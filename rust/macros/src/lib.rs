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
fn children(items: Vec<Item>) -> Result<Tokens> {
    let entries = items.into_iter().map(expand).collect::<Result<Vec<_>>>()?;
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
fn expand(item: Item) -> Result<Tokens> {
    let output = output();
    match item {
        Item::If { condition, yes, no } => {
            let yes = children(yes)?;
            let no = children(no)?;
            Ok(quote! { if #condition { #output.extend(#yes); } else { #output.extend(#no); } })
        }
        Item::For {
            pattern,
            values,
            body,
        } => {
            let body = children(body)?;
            Ok(quote! { for #pattern in #values { #output.extend(#body); } })
        }
        Item::Element {
            name,
            args,
            children: body,
        } => {
            let mut positional = Vec::new();
            let mut key = None;
            let mut action = None;
            for (property, value) in args {
                match property {
                    None => positional.push(value),
                    Some(property) => {
                        let slot = match property.to_string().as_str() {
                            "key" => &mut key,
                            "on_click" if name == "button" => &mut action,
                            _ => {
                                return Err(syn::Error::new(
                                    property.span(),
                                    "unsupported property in this stage",
                                ));
                            }
                        };
                        if slot.replace(value).is_some() {
                            return Err(syn::Error::new(property.span(), "duplicate property"));
                        }
                    }
                }
            }
            let kind = name.to_string();
            let value = match kind.as_str() {
                "column" | "row" => {
                    if !positional.is_empty() {
                        return Err(syn::Error::new(
                            name.span(),
                            "container arguments must be named properties",
                        ));
                    }
                    let body = children(body.ok_or_else(|| {
                        syn::Error::new(name.span(), "container needs a child block")
                    })?)?;
                    quote!(::scars_ui::View::#name(#body))
                }
                "label" | "button" => {
                    if body.is_some() {
                        return Err(syn::Error::new(
                            name.span(),
                            "this element cannot contain children",
                        ));
                    }
                    if positional.len() != 1 {
                        return Err(syn::Error::new(name.span(), "expected one text expression"));
                    }
                    let text = positional.remove(0);
                    if kind == "button" {
                        let action = action.ok_or_else(|| {
                            syn::Error::new(name.span(), "button needs on_click: handler")
                        })?;
                        quote!(::scars_ui::View::button(#text, #action))
                    } else {
                        quote!(::scars_ui::View::label(#text))
                    }
                }
                _ => {
                    if let Some(body) = body {
                        return Err(syn::Error::new(
                            name.span(),
                            if body.is_empty() {
                                "call a component function with parentheses"
                            } else {
                                "custom component functions cannot take child blocks yet"
                            },
                        ));
                    }
                    quote!(#name(#(#positional),*))
                }
            };
            let value = if let Some(key) = key {
                quote!((#value).key(#key))
            } else {
                value
            };
            Ok(quote! { #output.push(#value); })
        }
    }
}

/// Rust-like descriptions with expression arguments, `if`/`else`, and keyed `for` children.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let Tree(tree) = parse_macro_input!(input as Tree);
    match children(tree) {
        Ok(children) => quote!(::scars_ui::View::fragment(#children)).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
