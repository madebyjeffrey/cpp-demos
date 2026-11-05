use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{
    Expr, Ident, Pat, Result, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

enum Item {
    Element {
        name: Ident,
        args: Vec<(Option<Ident>, Expr)>,
        children: Option<Vec<Item>>,
    },
    If {
        condition: Expr,
        yes: Vec<Item>,
        no: Vec<Item>,
    },
    For {
        pattern: Pat,
        values: Expr,
        body: Vec<Item>,
    },
}
struct Tree(Vec<Item>);
impl Parse for Tree {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self(items(input)?))
    }
}
fn items(input: ParseStream<'_>) -> Result<Vec<Item>> {
    let mut result = Vec::new();
    while !input.is_empty() {
        result.push(item(input)?);
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        }
    }
    Ok(result)
}
fn block(input: ParseStream<'_>) -> Result<Vec<Item>> {
    let content;
    braced!(content in input);
    items(&content)
}
fn item(input: ParseStream<'_>) -> Result<Item> {
    if input.peek(Token![if]) {
        input.parse::<Token![if]>()?;
        let condition = input.call(Expr::parse_without_eager_brace)?;
        let yes = block(input)?;
        let no = if input.peek(Token![else]) {
            input.parse::<Token![else]>()?;
            if input.peek(Token![if]) {
                vec![item(input)?]
            } else {
                block(input)?
            }
        } else {
            Vec::new()
        };
        return Ok(Item::If { condition, yes, no });
    }
    if input.peek(Token![for]) {
        input.parse::<Token![for]>()?;
        let pattern = input.call(Pat::parse_single)?;
        input.parse::<Token![in]>()?;
        let values = input.call(Expr::parse_without_eager_brace)?;
        return Ok(Item::For {
            pattern,
            values,
            body: block(input)?,
        });
    }
    let name: Ident = input.parse()?;
    let mut args = Vec::new();
    if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);
        while !content.is_empty() {
            let property = if content.peek(Ident) && content.peek2(Token![:]) {
                let property = content.parse()?;
                content.parse::<Token![:]>()?;
                Some(property)
            } else {
                None
            };
            args.push((property, content.parse()?));
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
    }
    let children = if input.peek(syn::token::Brace) {
        Some(block(input)?)
    } else {
        None
    };
    Ok(Item::Element {
        name,
        args,
        children,
    })
}
fn children(items: Vec<Item>, root: &Tokens) -> Result<Tokens> {
    let entries = items
        .into_iter()
        .map(|item| expand(item, root))
        .collect::<Result<Vec<_>>>()?;
    let output = output();
    Ok(quote!({
        let mut #output = ::std::vec::Vec::new();
        #(#entries)*
        #output
    }))
}
fn output() -> Ident {
    Ident::new("__scars_children", proc_macro2::Span::mixed_site())
}
fn expand(item: Item, root: &Tokens) -> Result<Tokens> {
    let output = output();
    match item {
        Item::If { condition, yes, no } => {
            let yes = children(yes, root)?;
            let no = children(no, root)?;
            Ok(quote! {if #condition {#output.extend(#yes);}else{#output.extend(#no);}})
        }
        Item::For {
            pattern,
            values,
            body,
        } => {
            let body = children(body, root)?;
            Ok(quote! {for #pattern in #values {#output.extend(#body);}})
        }
        Item::Element {
            name,
            args,
            children: body,
        } => {
            let mut positional = Vec::new();
            let mut properties: Vec<(Ident, Expr)> = Vec::new();
            for (property, value) in args {
                if let Some(property) = property {
                    if properties.iter().any(|(p, _)| p == &property) {
                        return Err(syn::Error::new(property.span(), "duplicate property"));
                    }
                    properties.push((property, value));
                } else {
                    positional.push(value);
                }
            }
            let kind = name.to_string();
            let take = |props: &mut Vec<(Ident, Expr)>, key: &str| {
                props
                    .iter()
                    .position(|(p, _)| p == key)
                    .map(|i| props.remove(i).1)
            };
            let binding = take(&mut properties, "bind");
            let action = take(&mut properties, "on_click");
            let options = take(&mut properties, "options");
            let mode = take(&mut properties, "content_mode");
            let min = take(&mut properties, "min");
            let max = take(&mut properties, "max");
            let step = take(&mut properties, "step");
            let containers = [
                "column",
                "scroll",
                "row",
                "block",
                "inline",
                "layer",
                "panel",
                "section",
                "group_box",
            ];
            let is_container = containers.contains(&kind.as_str());
            if !is_container && body.is_some() {
                return Err(syn::Error::new(
                    name.span(),
                    "only containers take child blocks; call custom components as functions",
                ));
            }
            let one = |args: &mut Vec<Expr>| -> Result<Expr> {
                if args.len() != 1 {
                    return Err(syn::Error::new(
                        name.span(),
                        "expected one text/value expression",
                    ));
                }
                Ok(args.remove(0))
            };
            let need_binding = || {
                binding
                    .as_ref()
                    .map(|b| quote!((#b).binding()))
                    .ok_or_else(|| {
                        syn::Error::new(name.span(), "this control needs bind: state_or_binding")
                    })
            };
            let value = if is_container {
                let body = children(
                    body.ok_or_else(|| {
                        syn::Error::new(name.span(), "container needs a child block")
                    })?,
                    root,
                )?;
                if matches!(kind.as_str(), "section" | "group_box") {
                    let title = one(&mut positional)?;
                    quote!(#root::View::#name(#title,#body))
                } else {
                    if !positional.is_empty() {
                        return Err(syn::Error::new(
                            name.span(),
                            "container properties must be named",
                        ));
                    }
                    quote!(#root::View::#name(#body))
                }
            } else {
                match kind.as_str() {
                    "embed" => {
                        let value = one(&mut positional)?;
                        quote!(#value)
                    }
                    "rich_text" => {
                        let runs = one(&mut positional)?;
                        quote!(#root::View::rich_text(#runs))
                    }
                    "label" | "text" => {
                        let text = one(&mut positional)?;
                        quote!(#root::View::label(#text))
                    }
                    "button" | "command_button" => {
                        let text = one(&mut positional)?;
                        let action = action.as_ref().ok_or_else(|| {
                            syn::Error::new(name.span(), "button needs on_click: handler")
                        })?;
                        quote!(#root::View::button(#text,#action))
                    }
                    "checkbox" | "tri_checkbox" | "toggle" | "switch" | "text_field"
                    | "password_field" | "text_editor" | "search_field" => {
                        let text = one(&mut positional)?;
                        let b = need_binding()?;
                        quote!(#root::View::#name(#text,#b))
                    }
                    "slider" => {
                        let b = need_binding()?;
                        let min = min.as_ref().map_or(quote!(0.), |v| quote!((#v)as f64));
                        let max = max.as_ref().map_or(quote!(1.), |v| quote!((#v)as f64));
                        let step = step.as_ref().map_or(quote!(0.01), |v| quote!((#v)as f64));
                        quote!(#root::View::slider(#b,#min,#max,#step))
                    }
                    "progress" => {
                        if binding.is_some() {
                            let b = need_binding()?;
                            quote!(#root::View::progress(#b))
                        } else if positional.is_empty() {
                            quote!(#root::View::indeterminate_progress())
                        } else {
                            let v = one(&mut positional)?;
                            quote!({let value=(#v)as f64;#root::View::progress(#root::Binding::new(move||value,|_|{}))})
                        }
                    }
                    "radio_group" | "segmented" => {
                        let b = need_binding()?;
                        let options = options.as_ref().ok_or_else(|| {
                            syn::Error::new(
                                name.span(),
                                "selection control needs options: Vec<OptionItem>",
                            )
                        })?;
                        if kind == "radio_group" {
                            let title = one(&mut positional)?;
                            quote!(#root::View::radio_group(#title,#b,#options))
                        } else {
                            quote!(#root::View::segmented(#b,#options))
                        }
                    }
                    "image" => {
                        let image = one(&mut positional)?;
                        let mode = mode
                            .as_ref()
                            .map_or(quote!(#root::ContentMode::Fit), |v| quote!(#v));
                        quote!(#root::View::image((#image).clone(),#mode))
                    }
                    "separator" | "spacer" => quote!(#root::View::#name()),
                    _ => quote!(#name(#(#positional),*)),
                }
            };
            if matches!(
                kind.as_str(),
                "slider" | "segmented" | "separator" | "spacer"
            ) && !positional.is_empty()
            {
                return Err(syn::Error::new(
                    name.span(),
                    "this element only accepts named properties",
                ));
            }
            if binding.is_some()
                && !matches!(
                    kind.as_str(),
                    "checkbox"
                        | "tri_checkbox"
                        | "toggle"
                        | "switch"
                        | "text_field"
                        | "password_field"
                        | "text_editor"
                        | "search_field"
                        | "slider"
                        | "progress"
                        | "radio_group"
                        | "segmented"
                )
            {
                return Err(syn::Error::new(
                    name.span(),
                    "bind is not supported by this element",
                ));
            }
            if action.is_some() && !matches!(kind.as_str(), "button" | "command_button") {
                return Err(syn::Error::new(
                    name.span(),
                    "on_click is only supported by buttons",
                ));
            }
            if options.is_some() && !matches!(kind.as_str(), "radio_group" | "segmented") {
                return Err(syn::Error::new(
                    name.span(),
                    "options is only supported by selection controls",
                ));
            }
            if (min.is_some() || max.is_some() || step.is_some()) && kind != "slider" {
                return Err(syn::Error::new(
                    name.span(),
                    "min/max/step are slider properties",
                ));
            }
            if mode.is_some() && kind != "image" {
                return Err(syn::Error::new(
                    name.span(),
                    "content_mode is an image property",
                ));
            }
            let mut value = value;
            for (property, expr) in properties {
                let p = property.to_string();
                let expr = match p.as_str() {
                    "gap" | "grow" | "shrink" | "min_width" | "max_width" | "min_height"
                    | "max_height" | "font_size" | "radius" | "line_height" | "letter_spacing" => {
                        quote!((#expr)as f64)
                    }
                    "key" | "width" | "height" | "wrap" | "align" | "justify" | "bold"
                    | "foreground" | "background" | "enabled" | "visible" | "hidden"
                    | "position" | "left" | "right" | "top" | "bottom" | "z_index"
                    | "accessible_label" | "tooltip" | "on_context" | "autofocus" | "padding"
                    | "margin" | "basis" | "align_self" | "accent" | "scroll_axes"
                    | "scrollbars" | "density" | "text_wrap" | "text_overflow" | "text_align"
                    | "max_lines" | "label_style" | "uppercase" | "orientation" => quote!(#expr),
                    "max_length" | "undo_enabled" | "history_limit" | "read_only"
                        if matches!(
                            kind.as_str(),
                            "text_field"
                                | "password_field"
                                | "text_editor"
                                | "search_field"
                                | "slider"
                        ) =>
                    {
                        quote!(#expr)
                    }
                    "on_query" | "debounce" | "clear_button" if kind == "search_field" => {
                        quote!(#expr)
                    }
                    "allow_reveal" if kind == "password_field" => quote!(#expr),
                    "repeat" if matches!(kind.as_str(), "button" | "command_button") => {
                        quote!(#expr)
                    }
                    "on_submit" | "on_cancel" | "on_blur"
                        if matches!(
                            kind.as_str(),
                            "text_field" | "password_field" | "text_editor" | "search_field"
                        ) =>
                    {
                        quote!(#expr)
                    }
                    _ => {
                        return Err(syn::Error::new(
                            property.span(),
                            "unknown property for this element",
                        ));
                    }
                };
                value = quote!((#value).#property(#expr));
            }
            Ok(quote! {#output.push(#value);})
        }
    }
}

fn runtime_path() -> Tokens {
    use proc_macro_crate::{FoundCrate, crate_name};
    if let Ok(found) = crate_name("scars-ui") {
        return match found {
            FoundCrate::Itself => quote!(::scars_ui),
            FoundCrate::Name(name) => {
                let name = Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#name)
            }
        };
    }
    match crate_name("scars-window") {
        Ok(FoundCrate::Name(name)) => {
            let name = Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#name::ui)
        }
        _ => quote!(::scars_ui),
    }
}

/// Rust-like descriptions with expression arguments, `if`/`else`, and keyed `for` children.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let Tree(tree) = parse_macro_input!(input as Tree);
    let root = runtime_path();
    match children(tree, &root) {
        Ok(children) => quote!(#root::View::fragment(#children)).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
