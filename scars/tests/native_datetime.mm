#import <AppKit/AppKit.h>
#include <stdexcept>
namespace {
id held_date;
void check(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
NSWindow* window() {
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Date Time Test"])
            return candidate;
    return nil;
}
id label(id parent, NSString* text) {
    for (id child in [parent accessibilityChildren]) {
        if ([[child accessibilityLabel] isEqualToString:text])
            return child;
        if (id nested = label(child, text))
            return nested;
    }
    return nil;
}
} // namespace
void scars_datetime_native_step(int stage) {
    NSWindow* native = window();
    check(native != nil, "Missing native temporal test window");
    id client = native.contentView;
    id date = label(client, @"Departure date"), time = label(client, @"Departure time");
    check(date && time, "Native date/time fields expose accessible labels");
    if (stage == 0) {
        held_date = date;
        check([[date accessibilityRole] isEqualToString:NSAccessibilityTextFieldRole],
              "Date editors retain native text accessibility");
        [date setAccessibilityFocused:YES];
        [date setAccessibilityValue:@"02/29/2025"];
        [client doCommandBySelector:@selector(insertNewline:)];
        check([[date accessibilityHelp] length] > 0, "Invalid dates expose validation feedback");
    } else if (stage == 1) {
        [date setAccessibilityValue:@"03/01/2024"];
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 2) {
        [date setAccessibilitySelectedTextRange:NSMakeRange(0, [[date accessibilityValue] length])];
        [client setMarkedText:@"03/04/2024"
                selectedRange:NSMakeRange(10, 0)
             replacementRange:NSMakeRange(NSNotFound, 0)];
        check([client hasMarkedText], "Date fields retain the native marked-text client");
        const auto rect = [client firstRectForCharacterRange:NSMakeRange(0, 0) actualRange:nullptr];
        check(rect.size.height > 0, "Date fields provide candidate geometry");
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 3) {
        [client doCommandBySelector:@selector(insertNewline:)];
    } else if (stage == 4) {
        NSEvent* down = [NSEvent keyEventWithType:NSEventTypeKeyDown
                                         location:NSZeroPoint
                                    modifierFlags:NSEventModifierFlagOption
                                        timestamp:0
                                     windowNumber:native.windowNumber
                                          context:nil
                                       characters:@"\uF701"
                      charactersIgnoringModifiers:@"\uF701"
                                        isARepeat:NO
                                          keyCode:125];
        [client keyDown:down];
        id calendar = label(client, @"Calendar for Departure date");
        check(calendar && [date isAccessibilityExpanded], "Native Option-Down opens the calendar popover");
        id disabled = label(calendar, @"03/13/2024");
        check(disabled && ![disabled isAccessibilityEnabled],
              "Native calendar dates preserve disabled semantics");
        id selected = label(calendar, @"03/04/2024");
        check(selected && [selected isAccessibilitySelected],
              "Native calendar selection reflects the bound date");
        id target = label(calendar, @"03/05/2024");
        check(target && [target accessibilityPerformPress],
              "Accessible calendar days use the standard activation path");
    } else if (stage == 5) {
        [time setAccessibilityFocused:YES];
        [time setAccessibilitySelectedTextRange:NSMakeRange(3, 0)];
        [client doCommandBySelector:@selector(moveUp:)];
    } else if (stage == 6) {
        id increment = label(time, @"Increase time component");
        check(increment && [increment accessibilityPerformPress],
              "Native temporal steppers are accessible actions");
    } else if (stage == 7) {
        [time setAccessibilityFocused:YES];
        [time setAccessibilityValue:@"12:30"];
        id increment = label(time, @"Increase time component");
        check(increment && ![increment isAccessibilityEnabled] && ![increment accessibilityPerformPress],
              "Read-only time steppers are disabled");
    } else if(stage==8){
        id bounded=label(client,@"Bounded");
        id field=label(bounded,@"Bounded date");
        check(field!=nil,"Missing native composite date field");
        [field setAccessibilityFocused:YES];
        [field setAccessibilityValue:@"11/03/2026"];
        [client doCommandBySelector:@selector(insertNewline:)];
        check([[bounded accessibilityHelp] length]>0,"Joint date/time errors reach native accessibility");
    } else if(stage==9){
        id bounded=label(client,@"Bounded");
        id field=label(bounded,@"Bounded time");
        check(field!=nil,"Missing native composite time field");
        [field setAccessibilityFocused:YES];
        [field setAccessibilityValue:@"01:00"];
        [client doCommandBySelector:@selector(insertNewline:)];
        check([[bounded accessibilityHelp] length]==0,"Successful joint commits clear native validation feedback");
    }
}
void scars_datetime_native_closed() {
    check([[held_date accessibilityRole] isEqualToString:NSAccessibilityUnknownRole],
          "Retained date accessibility objects become inert after closure");
    held_date = nil;
}
