#import <AppKit/AppKit.h>
#include <stdexcept>
namespace {
void check(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
NSWindow* window(NSString* title) {
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:title])
            return candidate;
    return nil;
}
id find_label(id parent, NSString* label) {
    for (id child in [parent accessibilityChildren]) {
        if ([[child accessibilityLabel] isEqualToString:label])
            return child;
        if (id nested = find_label(child, label))
            return nested;
    }
    return nil;
}
} // namespace
void scars_inspect_alert_sheet() {
    NSWindow* sheet = window(@"SCARS Alert Sheet");
    NSWindow* parent = window(@"SCARS Alert Parent");
    NSWindow* other = window(@"SCARS Alert Other");
    check(sheet && sheet.sheetParent == parent && parent.attachedSheet == sheet && !other.attachedSheet,
          "The alert must be an actual sheet attached only to its parent");
    id content = sheet.contentView;
    id dialog = find_label(content, @"SCARS Alert Sheet");
    check(dialog && [[dialog accessibilitySubrole] isEqualToString:@"AXDialog"],
          "SCARS alert content exposes native dialog semantics");
    check([[[dialog accessibilityDefaultButton] accessibilityLabel] isEqualToString:@"Save"] &&
              [[[dialog accessibilityCancelButton] accessibilityLabel] isEqualToString:@"Cancel"],
          "The dialog exposes default and cancel button relationships");
    id save = find_label(content, @"Save");
    id remove = find_label(content, @"Delete");
    check(save && [[save accessibilityHelp] isEqualToString:@"Default action"] && remove &&
              [[remove accessibilityHelp] isEqualToString:@"Destructive action"],
          "Native accessibility preserves alert action roles");
    check(![find_label(parent.contentView, @"Parent action") isAccessibilityEnabled],
          "The modal parent's native actions expose disabled semantics");
    check(![find_label(parent.contentView, @"Parent action") accessibilityPerformPress],
          "Native parent actions are blocked while its sheet is open");
    check([find_label(other.contentView, @"Other action") accessibilityPerformPress],
          "An unrelated window remains interactive while a sheet is open");
    id client = sheet.contentView;
    [client setMarkedText:@"仮" selectedRange:NSMakeRange(1, 0) replacementRange:NSMakeRange(NSNotFound, 0)];
    check([client hasMarkedText], "Alert content retains the ordinary native input-method client");
    NSRect candidate = [client firstRectForCharacterRange:NSMakeRange(0, 0) actualRange:nullptr];
    check(candidate.size.height > 0, "The sheet editor supplies candidate-window geometry");
}
void scars_alert_command(bool escape) {
    NSWindow* sheet = window(@"SCARS Alert Parent").attachedSheet;
    check(sheet && [sheet.title isEqualToString:@"SCARS Alert Sheet"],
          "Missing attached alert sheet for native key action");
    if (escape) {
        NSEvent* event = [NSEvent keyEventWithType:NSEventTypeKeyDown
                                          location:NSZeroPoint
                                     modifierFlags:0
                                         timestamp:0
                                      windowNumber:sheet.windowNumber
                                           context:nil
                                        characters:@"\x1b"
                       charactersIgnoringModifiers:@"\x1b"
                                         isARepeat:NO
                                           keyCode:53];
        [sheet.contentView keyDown:event];
    } else {
        [sheet.contentView doCommandBySelector:@selector(insertNewline:)];
    }
}

void scars_close_alert_sheet() {
    NSWindow* sheet = window(@"SCARS Alert Parent").attachedSheet;
    check(sheet != nil, "Missing sheet for titlebar closure");
    // AppKit sheets suppress the traffic-light close control. Exercise the same
    // delegate close-request path used by native titlebar closure where offered.
    [sheet.delegate windowShouldClose:sheet];
}
void scars_check_alert_parent_restored() {
    NSWindow* parent = window(@"SCARS Alert Parent");
    check(!parent.attachedSheet && parent.firstResponder == parent.contentView,
          "Closing a sheet retains the parent's native first responder");
    check([find_label(parent.contentView, @"Parent action") isAccessibilityEnabled],
          "Closing a sheet restores parent accessibility interactivity");
}
