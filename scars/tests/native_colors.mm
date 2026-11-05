#import <AppKit/AppKit.h>
#include <stdexcept>
namespace {
id held;
void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
id label(id parent, NSString* text, NSString* role = nil) {
    for (id child in [parent accessibilityChildren]) {
        if ([[child accessibilityLabel] isEqualToString:text] &&
            (!role || [[child accessibilityRole] isEqualToString:role]))
            return child;
        if (id found = label(child, text, role))
            return found;
    }
    return nil;
}
NSWindow* window() {
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Color Test"])
            return candidate;
    return nil;
}
} // namespace
void scars_colors_native_step(int stage) {
    auto native = window();
    check(native != nil, "Missing native color test window");
    id client = native.contentView;
    id paint = label(client, @"Paint"), well = label(client, @"Preview");
    check(paint && well, "Native color labels exist");
    id hex = label(paint, @"Hex"), apply = label(paint, @"Apply");
    check(hex && apply, "Native color components are children of the picker");
    if (stage == 0) {
        held = hex;
        check([[hex accessibilityRole] isEqualToString:NSAccessibilityTextFieldRole],
              "Native hex uses the standard text role");
        [hex setAccessibilityFocused:YES];
        [hex setAccessibilityValue:@"#12G456"];
        check([apply accessibilityPerformPress], "Native invalid Apply action is routed");
        check([[paint accessibilityHelp] containsString:@"hex"],
              "Native color validation reaches the owning group");
    } else if (stage == 1) {
        [hex setAccessibilityFocused:YES];
        [hex setAccessibilityValue:@"#12345640"];
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 2) {
        check([apply accessibilityPerformPress], "Native Apply is accessible");
    } else if (stage == 3) {
        check([[well accessibilityValue] isEqualToString:@"#112233FF"],
              "Native well exposes CSS RGBA rather than packed ARGB");
        check([well accessibilityPerformPress], "Native ColorWell opens the picker");
        id red = label(well, @"Red", NSAccessibilityTextFieldRole);
        check(red && [well isAccessibilityExpanded], "Native well owns the expanded popup fields");
        [red setAccessibilityFocused:YES];
        [red setAccessibilityValue:@"200"];
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 4) {
        id red = label(well, @"Red", NSAccessibilityTextFieldRole);
        [red setAccessibilityFocused:YES];
        [red setAccessibilitySelectedTextRange:NSMakeRange(0, [[red accessibilityValue] length])];
        [client setMarkedText:@"220"
                selectedRange:NSMakeRange(3, 0)
             replacementRange:NSMakeRange(NSNotFound, 0)];
        check([client hasMarkedText], "Color numeric editors retain the marked-text client");
        const auto rect = [client firstRectForCharacterRange:NSMakeRange(0, 0) actualRange:nullptr];
        check(rect.size.height > 0, "Color fields provide native candidate geometry");
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 5) {
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 6) {
        [client doCommandBySelector:@selector(cancelOperation:)];
        check(![well isAccessibilityExpanded], "Escape closes a clean color popup");
    } else if (stage == 7) {
        [hex setAccessibilityFocused:YES];
        [hex setAccessibilityValue:@"#ffffff"];
        check(![apply isAccessibilityEnabled] && ![apply accessibilityPerformPress],
              "Read-only color Apply is disabled");
    }
}
void scars_colors_native_closed() {
    check([[held accessibilityRole] isEqualToString:NSAccessibilityUnknownRole],
          "Held native color editors become inert after closure");
    held = nil;
}
