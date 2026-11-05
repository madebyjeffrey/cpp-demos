#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#include <cmath>
#include <stdexcept>
#include <functional>
#include <iostream>
#include <string>
namespace {
void desktop_diagnostics() {
    uint32_t displays{};
    const auto error = CGGetActiveDisplayList(0, nullptr, &displays);
    auto foreground = NSWorkspace.sharedWorkspace.frontmostApplication;
    NSLog(@"SCARS desktop: active-displays=%u display-error=%d foreground=%@ app-active=%d key-window=%d",
          displays, int(error), foreground.bundleIdentifier ?: @"none", NSApp.active,
          NSApp.keyWindow != nil);
}
void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
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
void scars_scroll_native(int vertical, int horizontal) {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Wheel Test"])
            window = candidate;
    check(window != nil, "Missing native SCARS wheel test window");
    auto wheel = CGEventCreateScrollWheelEvent(nullptr, kCGScrollEventUnitPixel, 2, vertical, horizontal);
    check(wheel != nullptr, "Could not create native wheel event");
    // Dispatch directly to the test view. NSEvents without a window express this
    // location in screen coordinates; choose a point inside the view either way.
    CGEventSetLocation(wheel, CGPointMake(20, NSMaxY(NSScreen.screens.firstObject.frame) - 20));
    auto event = [NSEvent eventWithCGEvent:wheel];
    CFRelease(wheel);
    check(event.type == NSEventTypeScrollWheel, "Expected a native scroll-wheel event");
    [window.contentView scrollWheel:event];
}
bool scars_focus_lifetime_password(bool allow_existing) {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Input Lifetime Test"]) window = candidate;
    check(window != nil, "Missing password lifetime window");
    if (!NSApp.active || !window.keyWindow || (!allow_existing && IsSecureEventInputEnabled())) return false;
    id password = find_label(window.contentView, @"Lifetime password");
    check(password != nil, "Missing password lifetime field");
    [password setAccessibilityFocused:YES];
    check(IsSecureEventInputEnabled(), "Native lifetime password did not enable secure input");
    return true;
}
void scars_check_secure_input_off() { check(!IsSecureEventInputEnabled(), "Native password lifetime leaked secure input"); }
void scars_inspect_virtual_list(const std::function<void()>& scroll_away) {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Virtual List Test"]) window = candidate;
    check(window != nil, "Missing native virtual list window");
    NSView<NSTextInputClient>* view = (id)window.contentView;
    id list = [view accessibilityChildren].firstObject;
    check([[list accessibilityRole] isEqualToString:NSAccessibilityListRole] && [list accessibilityRowCount] == 100000,
          "Native logical row count is incorrect");
    NSArray* rows = [list accessibilityRows];
    check(rows.count == 100000 && [[list accessibilityChildren] count] == 100000,
          "Native row arrays do not represent the logical dataset");
    id far = rows[99999];
    check([far accessibilityIndex] == 99999 && [[far accessibilityLabel] isEqualToString:@"Record 99999"] &&
          NSEqualRects([far accessibilityFrame], NSZeroRect), "Offscreen logical rows expose incorrect metadata or fabricated frames");
    id first = rows[0];
    id editor = find_label(first, @"Native virtual editor");
    check(editor != nil, "Missing realized virtual row editor");
    [editor setAccessibilityFocused:YES];
    [view setMarkedText:@"仮" selectedRange:NSMakeRange(1,0) replacementRange:NSMakeRange(NSNotFound,0)];
    scroll_away();
    check(view.hasMarkedText && NSEqualRects([editor accessibilityFrame], NSZeroRect), "Native composition was not retained outside the viewport");
    [view unmarkText];
    [far setAccessibilityFocused:YES];
    check([far isAccessibilityFocused] && !NSEqualRects([far accessibilityFrame], NSZeroRect), "Logical accessibility focus did not reveal the distant row");
    check([far accessibilityPerformPress] && [far isAccessibilitySelected], "Offscreen accessibility selection failed");
    NSArray* selected = [list accessibilitySelectedRows];
    check(selected.count == 1 && [selected[0] accessibilityIndex] == 99999, "Native selected rows lost their logical index");
    check([[list accessibilityVisibleRows] count] <= 8, "Native visible rows included unrealized data");
}
namespace {
id saved_native_table, saved_native_cell, saved_native_column;
NSArray* saved_native_rows;
NSArray* saved_native_selection;
}
void scars_check_table_closed() {
    check([[saved_native_table accessibilityRole] isEqualToString:NSAccessibilityUnknownRole]&&
          [[saved_native_cell accessibilityRole] isEqualToString:NSAccessibilityUnknownRole]&&
          [[saved_native_column accessibilityRole] isEqualToString:NSAccessibilityUnknownRole]&&saved_native_rows.count==0,
          "Closed native table retained live accessibility state");
    check(saved_native_selection.count==2&&[[saved_native_selection[0] accessibilityRole] isEqualToString:NSAccessibilityUnknownRole],
          "Held selection snapshot became unsafe after table removal/close");
    saved_native_table=nil;saved_native_cell=nil;saved_native_column=nil;saved_native_rows=nil;saved_native_selection=nil;
}
void scars_inspect_table(const std::function<void(int)>& change) {
    struct RestoreClipboard {
        NSPasteboard* __strong board=NSPasteboard.generalPasteboard;
        NSMutableArray<NSPasteboardItem*>* __strong saved=[NSMutableArray array];
        RestoreClipboard(){for(NSPasteboardItem* item in board.pasteboardItems){auto clone=[NSPasteboardItem new];for(NSPasteboardType type in item.types)if(NSData* data=[item dataForType:type])[clone setData:data forType:type];[saved addObject:clone];}}
        ~RestoreClipboard(){[board clearContents];if(saved.count)[board writeObjects:saved];}
    } restore;
    NSWindow* window=nil;
    for(NSWindow* candidate in NSApp.windows)if([candidate.title isEqualToString:@"SCARS Table Test"])window=candidate;
    check(window!=nil,"Missing native table window");
    NSView<NSTextInputClient>* view=(id)window.contentView;
    id table=[view accessibilityChildren].firstObject;
    check([[table accessibilityRole] isEqualToString:NSAccessibilityTableRole]&&[table accessibilityRowCount]==100000&&[table accessibilityColumnCount]==3,"Native table dimensions or role are incorrect");
    NSArray* columns=[table accessibilityColumns];NSArray* rows=[table accessibilityRows];
    check(columns.count==3&&rows.count==100000,"Native table arrays do not represent logical dimensions");
    id column=columns[1];id farRow=rows[99999];id firstRow=rows[0];
    id far=[farRow accessibilityChildren][1];id first=[firstRow accessibilityChildren][1];
    check([[column accessibilityRole] isEqualToString:NSAccessibilityColumnRole]&&[column accessibilityIndex]==1&&[[column accessibilityChildren] count]==100000,"Native columns are not lazy indexed column objects");
    check([[far accessibilityRole] isEqualToString:NSAccessibilityCellRole]&&[far accessibilityRowIndexRange].location==99999&&[far accessibilityColumnIndexRange].location==1,"Native cell indices are incorrect");
    check([[[table accessibilityCellForColumn:1 row:99999] accessibilityIdentifier] isEqualToString:[far accessibilityIdentifier]]&&
          [[[[column accessibilityChildren] objectAtIndex:99999] accessibilityIdentifier] isEqualToString:[far accessibilityIdentifier]],"Native row, column, and coordinate access disagree about cell identity");
    check(NSEqualRects([far accessibilityFrame],NSZeroRect)&&[table accessibilityCellForColumn:-1 row:0]==nil&&[table accessibilityCellForColumn:3 row:0]==nil,"Native table invented offscreen geometry or accepted invalid coordinates");
    id header=[column accessibilityHeader];id group=[table accessibilityHeader];
    check([[table accessibilityVisibleColumns] count]<=3&&[[table accessibilityColumnHeaderUIElements] count]==3,"Native visible-column or header arrays are incorrect");
    id resize=find_label(header,@"Resize Value");
    check(resize&&[[resize accessibilityRole] isEqualToString:NSAccessibilitySplitterRole],"Native column resize handle is missing");
    [resize setAccessibilityValue:@200];check([[resize accessibilityValue] doubleValue]==200&&[resize accessibilityPerformIncrement]&&[[resize accessibilityValue] doubleValue]==210,"Native bounded column resizing or adjustment failed");
    check(header&&group&&[[group accessibilityChildren] count]==3&&[[header accessibilitySubrole] isEqualToString:NSAccessibilitySortButtonSubrole],"Native table header relationships or sort-button semantics are incorrect");
    check([header accessibilityPerformPress]&&[header accessibilitySortDirection]==NSAccessibilitySortDirectionAscending&&[column accessibilitySortDirection]==NSAccessibilitySortDirectionAscending,"Native sort requests do not update descriptor metadata");
    [table setAccessibilitySelectedCells:@[first,far]];
    saved_native_selection=[table accessibilitySelectedCells];
    check([[table accessibilitySelectedCells] count]==2&&[[table accessibilitySelectedRows] count]==0&&[far isAccessibilitySelected],"Native batch cell selection failed");
    [far setAccessibilitySelected:NO];check(![far isAccessibilitySelected]&&[[table accessibilitySelectedCells] count]==1,"Native cell deselection reselected the cell");
    [far setAccessibilitySelected:YES];check([[table accessibilitySelectedCells] count]==2,"Native additive cell selection cleared another selection");
    id disabled=[table accessibilityCellForColumn:1 row:1];[table setAccessibilitySelectedCells:@[disabled]];
    check([[table accessibilitySelectedCells] count]==2,"Native invalid batch selection partially changed state");
    [far setAccessibilityFocused:YES];
    check([far isAccessibilityFocused]&&!NSEqualRects([far accessibilityFrame],NSZeroRect)&&
          [[[view accessibilityFocusedUIElement] accessibilityIdentifier] isEqualToString:[far accessibilityIdentifier]],"Native cell focus did not reveal the distant cell");
    check([[table accessibilityVisibleCells] count]<=30,"Native visible cells include unrealized rows");
    check([far accessibilityPerformConfirm],"Native cell confirmation did not begin editing");
    [view insertText:@"42" replacementRange:NSMakeRange(NSNotFound,0)];
    check([[far accessibilityValue] isEqualToString:@"42"],"Native cell did not expose its current draft");
    [view doCommandBySelector:NSSelectorFromString(@"selectAll:")];[view doCommandBySelector:NSSelectorFromString(@"copy:")];
    check([[NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString] isEqualToString:@"42"],"Native table copy did not use the system clipboard");
    [view doCommandBySelector:NSSelectorFromString(@"cut:")];check([[far accessibilityValue] isEqualToString:@""],"Native table cut did not change its draft");
    [view doCommandBySelector:NSSelectorFromString(@"paste:")];[view doCommandBySelector:NSSelectorFromString(@"undo:")];
    check([[far accessibilityValue] isEqualToString:@""],"Native table draft undo failed");
    [view doCommandBySelector:NSSelectorFromString(@"redo:")];check([[far accessibilityValue] isEqualToString:@"42"],"Native table draft redo failed");change(5);
    NSRange actual;check([view firstRectForCharacterRange:view.selectedRange actualRange:&actual].size.height>0,"Native table editor has no candidate geometry");
    [view doCommandBySelector:NSSelectorFromString(@"insertNewline:")];change(6);
    check([[far accessibilityValue] isEqualToString:@"42"],"Native table commit did not update cell data");
    [far setAccessibilityValue:@"bad"];change(6);
    check([[far accessibilityHelp] isEqualToString:@"Use digits"],"Native table validation feedback is missing");
    [view doCommandBySelector:NSSelectorFromString(@"cancelOperation:")];
    check([far accessibilityPerformConfirm],"Native table editor could not reopen");
    [view setMarkedText:@"仮" selectedRange:NSMakeRange(1,0) replacementRange:NSMakeRange(NSNotFound,0)];change(0);
    check(view.hasMarkedText&&NSEqualRects([far accessibilityFrame],NSZeroRect),"Native table virtualization lost active composition");
    [far setAccessibilityValue:@"55"];check(view.hasMarkedText,"Native accessibility value replacement interrupted composition");
    [view doCommandBySelector:NSSelectorFromString(@"cancelOperation:")];check(!view.hasMarkedText,"Native first Escape did not cancel composition");
    [view doCommandBySelector:NSSelectorFromString(@"cancelOperation:")];
    id previous=[table accessibilityCellForColumn:1 row:99997];[previous setAccessibilityFocused:YES];
    unichar functionKey=NSF2FunctionKey;
    auto event=[NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:0 timestamp:0 windowNumber:window.windowNumber context:nil
        characters:[NSString stringWithCharacters:&functionKey length:1] charactersIgnoringModifiers:[NSString stringWithCharacters:&functionKey length:1] isARepeat:NO keyCode:120];
    [view keyDown:event];
    check(view.selectedRange.location==0&&view.selectedRange.length==2,"Native F2 did not enter the cell editor");
    [view insertText:@"43" replacementRange:NSMakeRange(NSNotFound,0)];
    [view doCommandBySelector:NSSelectorFromString(@"insertTab:")];
    check([[[view accessibilityFocusedUIElement] accessibilityIdentifier] containsString:@"option/99998/cell/value"],"Native Tab did not commit and traverse logical editable cells");
    [view doCommandBySelector:NSSelectorFromString(@"cancelOperation:")];change(7);
    change(1);
    check([[table accessibilitySelectedRows] count]==1&&[[table accessibilitySelectedCells] count]==3,"Row selection did not expose its cells lazily");
    [table setAccessibilitySelectedRows:@[firstRow,farRow]];
    check([[table accessibilitySelectedRows] count]==2&&[[table accessibilitySelectedCells] count]==6,"Native batch row selection failed");
    [farRow setAccessibilitySelected:NO];check([[table accessibilitySelectedRows] count]==1,"Native row deselection failed");
    change(2);
    check([column accessibilityIndex]==0&&[far accessibilityColumnIndexRange].location==0&&
        [[[table accessibilityCellForColumn:0 row:99999] accessibilityIdentifier] isEqualToString:[far accessibilityIdentifier]],"Held native cell and column objects lost identity after reordering");
    check([far accessibilityPerformConfirm],"Reordered native cell could not enter editing");
    [view setMarkedText:@"仮" selectedRange:NSMakeRange(1,0) replacementRange:NSMakeRange(NSNotFound,0)];change(3);
    check(!view.hasMarkedText&&[[column accessibilityRole] isEqualToString:NSAccessibilityUnknownRole]&&[[far accessibilityRole] isEqualToString:NSAccessibilityUnknownRole]&&[table accessibilityColumnCount]==2,"Native schema removal retained stale editor or column objects");
    saved_native_table=table;saved_native_rows=rows;saved_native_cell=[table accessibilityCellForColumn:0 row:99999];saved_native_column=[table accessibilityColumns][0];
}
void scars_inspect_tree(const std::function<void()>& replace_model) {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Tree Test"]) window = candidate;
    check(window != nil, "Missing native tree window");
    id outline = [window.contentView accessibilityChildren].firstObject;
    check([[outline accessibilityRole] isEqualToString:NSAccessibilityOutlineRole] && [outline accessibilityRowCount] == 2,
          "Native outline role or collapsed row count is incorrect");
    id branch = [outline accessibilityRows][0];
    check([branch accessibilityDisclosureLevel] == 0 && ![branch isAccessibilityDisclosed], "Native root disclosure metadata is incorrect");
    [branch setAccessibilityDisclosed:YES];
    check([branch isAccessibilityExpanded] && [outline accessibilityRowCount] == 102, "Native expansion did not flatten lazy children");
    NSArray* children = [branch accessibilityDisclosedRows];
    check(children.count == 100, "Native disclosed rows do not expose logical immediate children");
    id far = children[99];
    check([[far accessibilityLabel] isEqualToString:@"Native node 99"] && [far accessibilityDisclosureLevel] == 1 &&
          [[[far accessibilityDisclosedByRow] accessibilityIdentifier] isEqualToString:[branch accessibilityIdentifier]] &&
          NSEqualRects([far accessibilityFrame],NSZeroRect), "Native child hierarchy or offscreen geometry is incorrect");
    [far setAccessibilityFocused:YES];
    check([far accessibilityPerformPress] && [far isAccessibilitySelected] && !NSEqualRects([far accessibilityFrame],NSZeroRect),
          "Native tree focus/selection did not reveal its distant logical row");
    [branch setAccessibilityExpanded:NO];
    check([outline accessibilityRowCount] == 2 && [[branch accessibilityDisclosedRows] count] == 0,
          "Native collapse retained hidden descendants");
    [branch setAccessibilityExpanded:YES];
    check([[branch accessibilityDisclosedRows] count] == 100 && [far isAccessibilitySelected], "Native expansion lost cached child identity or selection");
    id loading = [outline accessibilityRows][101];
    [loading setAccessibilityExpanded:YES];
    check([[loading accessibilityHelp] isEqualToString:@"Loading children"] && [[loading accessibilityDisclosedRows] count] == 0,
          "Native loading state invented children or lost its help text");
    replace_model();
    check([outline accessibilityRowCount] == 2 && [[branch accessibilityLabel] isEqualToString:@"Native renamed fleet"] &&
          [[far accessibilityRole] isEqualToString:NSAccessibilityUnknownRole], "Native model replacement retained stale logical row objects");
}
void scars_inspect_native() {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Native Test"])
            window = candidate;
    check(window != nil, "Missing native SCARS window");
    // Match the existing desktop tests: keep the measured canvas exposed even if
    // macOS delays or declines programmatic application activation.
    window.collectionBehavior |=
        NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary;
    [window makeKeyAndOrderFront:nil];
    NSView* view = window.contentView;
    check([view.accessibilityRole isEqualToString:NSAccessibilityGroupRole],
          "SCARS native root is not accessible");
    id checkbox = find_label(view, @"Native checkbox");
    check(checkbox != nil, "Missing native checkbox");
    check([[checkbox accessibilityRole] isEqualToString:NSAccessibilityCheckBoxRole],
          "Incorrect checkbox role");
    check([checkbox accessibilityPerformPress], "Native checkbox press failed");
    check([[checkbox accessibilityValue] boolValue], "Native checkbox value is stale");
    [checkbox setAccessibilityValue:@0];
    NSPoint point = [view convertPoint:NSMakePoint(40, 38) toView:nil];
    for (auto type : {NSEventTypeLeftMouseDown, NSEventTypeLeftMouseUp}) {
        auto event = [NSEvent mouseEventWithType:type
                                        location:point
                                   modifierFlags:0
                                       timestamp:0
                                    windowNumber:window.windowNumber
                                         context:nil
                                     eventNumber:0
                                      clickCount:1
                                        pressure:0];
        if (type == NSEventTypeLeftMouseDown)
            [view mouseDown:event];
        else
            [view mouseUp:event];
    }
    check([[checkbox accessibilityValue] boolValue], "Native pointer events did not reach SCARS");
    id radio = find_label(view, @"Native radio B");
    check([[radio accessibilityRole] isEqualToString:NSAccessibilityRadioButtonRole], "Incorrect radio role");
    check([radio accessibilityParent] != view, "Radio group association missing");
    check([radio accessibilityPerformPress], "Native radio press failed");
    id slider = find_label(view, @"Native slider");
    check([[slider accessibilityRole] isEqualToString:NSAccessibilitySliderRole], "Incorrect slider role");
    [slider setAccessibilityValue:@(.6)];
    check([slider accessibilityPerformIncrement], "Native slider increment failed");
    id divider = find_label(view, @"Native panes divider");
    check(divider && [[divider accessibilityRole] isEqualToString:NSAccessibilitySplitterRole],
          "Incorrect split divider role");
    check([divider accessibilityOrientation] == NSAccessibilityOrientationVertical,
          "A horizontal split should expose a vertical divider");
    [divider setAccessibilityValue:@(.6)];
    check(std::abs([[divider accessibilityValue] doubleValue] - .6) < .001,
          "Native split value did not update");
    check([divider accessibilityPerformIncrement] && [[divider accessibilityValue] doubleValue] > .6,
          "Native split increment failed");
    check([divider accessibilityParent] != view, "Split divider parent group association missing");
    id pages = find_label(view, @"Native pages");
    check(pages && [[pages accessibilityRole] isEqualToString:NSAccessibilityTabGroupRole],
          "Missing native tab group semantics");
    id tab_a = find_label(view, @"Native tab A"), tab_b = find_label(view, @"Native tab B");
    check([[tab_a accessibilityRole] isEqualToString:NSAccessibilityRadioButtonRole] &&
              [tab_a accessibilityParent] == pages && [tab_a isAccessibilitySelected],
          "Native tab role, parent, or selection is incorrect");
    check([tab_b accessibilityPerformPress] && [tab_b isAccessibilitySelected] &&
              find_label(view, @"Native text") == nil,
          "Native tab selection did not hide inactive page accessibility");
    id amount = find_label(view, @"Native amount"), count = find_label(view, @"Native count"),
       toggle = find_label(view, @"Native switch");
    check([[amount accessibilityRole] isEqualToString:NSAccessibilityTextFieldRole], "Numeric field is missing text input semantics");
    [amount setAccessibilityValue:@"4.5"];
    [amount setAccessibilityFocused:YES];
    [view doCommandBySelector:@selector(insertNewline:)];
    check([[amount accessibilityValue] isEqualToString:@"4.5"] && [amount accessibilityPerformIncrement] &&
              [[amount accessibilityValue] isEqualToString:@"4.75"], "Native numeric commit or increment failed");
    [amount setAccessibilityValue:@"invalid"];
    [view doCommandBySelector:@selector(insertNewline:)];
    check([[amount accessibilityHelp] length] > 0, "Numeric validation help did not reach accessibility");
    [view doCommandBySelector:@selector(cancelOperation:)];
    check([[amount accessibilityValue] isEqualToString:@"4.75"], "Native Escape did not restore committed numeric text");
    check([[count accessibilityRole] isEqualToString:NSAccessibilityIncrementorRole] &&
              [count accessibilityPerformIncrement] && [[count accessibilityValue] intValue] == 4,
          "Native stepper semantics or increment failed");
    check([[toggle accessibilitySubrole] isEqualToString:NSAccessibilitySwitchSubrole] &&
              [toggle accessibilityPerformPress] && [[toggle accessibilityValue] boolValue],
          "Native switch semantics or activation failed");
    id credentials = find_label(view, @"Native credentials");
    check([credentials accessibilityPerformPress], "Native credentials page could not be opened");
    id search = find_label(view, @"Native search"), password = find_label(view, @"Native password");
    check([[search accessibilitySubrole] isEqualToString:NSAccessibilitySearchFieldSubrole] &&
              [[password accessibilitySubrole] isEqualToString:NSAccessibilitySecureTextFieldSubrole],
          "Search or password accessibility subrole is missing");
    [search setAccessibilityValue:@"needle"];
    check([find_label(view, @"Clear search") accessibilityPerformPress] &&
              [[search accessibilityValue] isEqualToString:@""], "Native search clear action failed");
    check([[search accessibilityHelp] isEqualToString:@"Search help"], "Tooltip help did not reach native accessibility");
    [search setAccessibilityFocused:YES];
    [view doCommandBySelector:@selector(insertNewline:)];
    id overlay_editor = find_label(view, @"Native overlay editor");
    check(overlay_editor && [overlay_editor isAccessibilityFocused] &&
              [[[overlay_editor accessibilityParent] accessibilityRole] isEqualToString:NSAccessibilityGroupRole],
          "Popover content did not expose focused native controls and group semantics");
    [overlay_editor setAccessibilityValue:@"Overlay edited"];
    check([[overlay_editor accessibilityValue] isEqualToString:@"Overlay edited"], "Native popover text editing failed");
    [view doCommandBySelector:@selector(cancelOperation:)];
    check(find_label(view, @"Native overlay editor") == nil, "Native Escape failed to dismiss popover");
    check([search isAccessibilityFocused], "Native popover dismissal failed to restore editor focus");
    auto context_key = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:NSEventModifierFlagShift
                                  timestamp:0 windowNumber:window.windowNumber context:nil
                                 characters:[NSString stringWithFormat:@"%C", static_cast<unichar>(NSF10FunctionKey)]
                charactersIgnoringModifiers:[NSString stringWithFormat:@"%C", static_cast<unichar>(NSF10FunctionKey)] isARepeat:NO keyCode:109];
    [view keyDown:context_key];
    id pinned = find_label(view, @"Native pin search");
    check(pinned && [[pinned accessibilityRole] isEqualToString:NSAccessibilityMenuItemRole] &&
              [[[pinned accessibilityParent] accessibilityRole] isEqualToString:NSAccessibilityMenuRole],
          "Native Shift-F10 did not expose a context menu and menu item roles");
    check([pinned accessibilityPerformPress], "Native checked context action failed");
    const NSRect search_frame = [search accessibilityFrame];
    const NSPoint context_point = [window convertPointFromScreen:NSMakePoint(NSMidX(search_frame), NSMidY(search_frame))];
    auto context_mouse = [&](NSEventType type, NSEventModifierFlags flags) {
        NSEvent* event = [NSEvent mouseEventWithType:type location:context_point modifierFlags:flags timestamp:0
                                      windowNumber:window.windowNumber context:nil eventNumber:0 clickCount:1 pressure:1];
        // AppKit's convenience constructor supplies button zero even for a
        // right-mouse type. Preserve its window/coordinates and correct the
        // underlying event to match an actual secondary-button input.
        if (type == NSEventTypeRightMouseDown || type == NSEventTypeRightMouseUp) {
            auto cg = CGEventCreateCopy(event.CGEvent);
            check(cg != nullptr, "Could not construct native secondary-button event");
            CGEventSetIntegerValueField(cg, kCGMouseEventButtonNumber, kCGMouseButtonRight);
            event = [NSEvent eventWithCGEvent:cg];
            CFRelease(cg);
            check(event.windowNumber == window.windowNumber, "Native context event lost its window association");
        }
        return event;
    };
    auto right_down = context_mouse(NSEventTypeRightMouseDown, 0);
    check(right_down.buttonNumber == 1, "Synthetic native context event has the wrong button number");
    [view rightMouseDown:right_down];
    [view rightMouseUp:context_mouse(NSEventTypeRightMouseUp, 0)];
    pinned = find_label(view, @"Native pin search");
    check(pinned != nil, "Native right-click did not invoke the attached context menu");
    check([pinned isAccessibilitySelected], "Native checked context-menu state failed");
    check([[pinned accessibilityValue] isEqualToString:@"Checked"], "Native checked context-menu value failed");
    check([find_label(view, @"Native more actions") accessibilityPerformPress] &&
              find_label(view, @"Native context action") != nil,
          "Native submenu activation did not expose its children");
    auto escape_context = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:0 timestamp:0
                                        windowNumber:window.windowNumber context:nil characters:@"\033"
                       charactersIgnoringModifiers:@"\033" isARepeat:NO keyCode:53];
    [view keyDown:escape_context];
    check(find_label(view, @"Native pin search") != nil && find_label(view, @"Native context action") == nil,
          "Native Escape closed more than the top context submenu");
    [view keyDown:escape_context];
    check(find_label(view, @"Native pin search") == nil && [search isAccessibilityFocused],
          "Native context dismissal failed to restore text focus");
    [view mouseDown:context_mouse(NSEventTypeLeftMouseDown, NSEventModifierFlagControl)];
    [view mouseUp:context_mouse(NSEventTypeLeftMouseUp, NSEventModifierFlagControl)];
    check(find_label(view, @"Native pin search") != nil, "Native Control-click failed to invoke a context menu");
    check([find_label(view, @"Native more actions") accessibilityPerformPress] &&
              [find_label(view, @"Native context action") accessibilityPerformPress] &&
              find_label(view, @"Native pin search") == nil,
          "Native nested context acceptance failed to dismiss the menu chain");
    [password setAccessibilityValue:@"secret"];
    const bool check_secure_input = NSApp.active && window.keyWindow && !IsSecureEventInputEnabled();
    if (!check_secure_input) {
        desktop_diagnostics();
        std::cout << "Secure-input ownership check not eligible: application active=" << bool(NSApp.active)
                  << ", key window=" << bool(window.keyWindow) << ", prior secure input=" << bool(IsSecureEventInputEnabled()) << '\n';
    }
    [password setAccessibilityFocused:YES];
    if (check_secure_input) check(IsSecureEventInputEnabled(), "Password focus did not enable secure event input");
    check([[password accessibilityValue] isEqualToString:@"••••••"] &&
              [[password accessibilitySelectedText] length] == 0 &&
              [password accessibilitySelectedTextRange].location == NSNotFound,
          "Password accessibility exposed plaintext or selection");
    id<NSTextInputClient> input = (id<NSTextInputClient>)view;
    NSRange actual;
    check([input attributedSubstringForProposedRange:NSMakeRange(0, 6) actualRange:&actual] == nil,
          "Native text queries exposed a password substring");
    [NSPasteboard.generalPasteboard clearContents];
    [NSPasteboard.generalPasteboard setString:@"clipboard sentinel" forType:NSPasteboardTypeString];
    [view doCommandBySelector:@selector(selectAll:)];
    [view doCommandBySelector:@selector(copy:)]; [view doCommandBySelector:@selector(cut:)];
    check([[NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString] isEqualToString:@"clipboard sentinel"] &&
              [[password accessibilityValue] isEqualToString:@"••••••"], "Password copy/cut leaked or deleted its value");
    check([find_label(view, @"Show password") accessibilityPerformPress] &&
              [[password accessibilityValue] isEqualToString:@"••••••"] &&
              [input attributedSubstringForProposedRange:NSMakeRange(0, 6) actualRange:&actual] == nil,
          "Visual password reveal weakened native redaction");
    auto find_event = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:NSEventModifierFlagCommand
                                     timestamp:0 windowNumber:window.windowNumber context:nil characters:@"f"
                    charactersIgnoringModifiers:@"f" isARepeat:NO keyCode:3];
    [view keyDown:find_event];
    check([search isAccessibilityFocused], "Native Command-F did not focus search from a password field");
    if (check_secure_input) check(!IsSecureEventInputEnabled(), "Leaving password focus leaked secure event input ownership");
    if (check_secure_input) std::cout << "Secure-input ownership enabled and released with password focus.\n";
    check([find_label(view, @"Native choices") accessibilityPerformPress], "Native combo page could not be selected");
    id combo = find_label(view, @"Native combo");
    check(combo && [[combo accessibilityRole] isEqualToString:NSAccessibilityComboBoxRole], "Combo accessibility role is missing");
    [combo setAccessibilityFocused:YES]; [combo setAccessibilityValue:@"Be"];
    id suggestion = find_label(view, @"Beta");
    check(suggestion && [[suggestion accessibilityRole] isEqualToString:NSAccessibilityRowRole] &&
              [combo isAccessibilityFocused] && [combo isAccessibilityExpanded],
          "Combo filtering did not expose suggestions while preserving native editor focus");
    [suggestion setAccessibilityFocused:YES];
    check([combo isAccessibilityFocused] && [suggestion accessibilityPerformPress] &&
              [[combo accessibilityValue] isEqualToString:@"Beta"] && ![combo isAccessibilityExpanded],
          "Native combo acceptance or editing-focus retention failed");
    [combo setAccessibilityValue:@"Unmatched"];
    [view doCommandBySelector:@selector(cancelOperation:)];
    check([[combo accessibilityValue] isEqualToString:@"Beta"] && ![combo isAccessibilityExpanded],
          "Native ComboBox Escape did not restore its accepted query");
    check([tab_a accessibilityPerformPress], "Native first tab could not be restored");
    id tab_text = find_label(view, @"Native text");
    check(tab_text && [[tab_text accessibilityParent] accessibilityParent] == pages,
          "Native page controls are missing their tab panel parent");
    [tab_text setAccessibilityFocused:YES];
    for (int pass = 0; pass < 2; ++pass) {
        const NSEventModifierFlags flags = NSEventModifierFlagControl |
            (pass ? NSEventModifierFlagShift : NSEventModifierFlags(0));
        auto event = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:flags
                                   timestamp:0 windowNumber:window.windowNumber context:nil
                                  characters:@"\t" charactersIgnoringModifiers:@"\t" isARepeat:NO keyCode:48];
        [view keyDown:event];
        if (!(flags & NSEventModifierFlagShift))
            check([tab_b isAccessibilitySelected], "Native Control-Tab did not switch from an active editor");
    }
    check([tab_a isAccessibilitySelected] &&
              [[find_label(view, @"Native text") accessibilityValue] isEqualToString:@"initial"],
          "Native tab shortcuts inserted text or failed to restore the first page");
    id selection = find_label(view, @"Native selection");
    check([[selection accessibilityRole] isEqualToString:NSAccessibilityPopUpButtonRole],
          "Incorrect dropdown role");
    check([selection accessibilityPerformPress], "Native dropdown open failed");
    id choice = find_label(view, @"Native value B");
    check([[choice accessibilityRole] isEqualToString:NSAccessibilityMenuItemRole],
          "Missing popup menu item semantics");
    check([choice accessibilityPerformPress], "Native selection failed");
    id arrow = find_label(view, @"Choose action");
    check([[arrow accessibilityRole] isEqualToString:NSAccessibilityMenuButtonRole],
          "Missing split arrow role");
    check([arrow accessibilityPerformPress], "Native split arrow failed");
    id action = find_label(view, @"Native action B");
    check([action accessibilityPerformPress], "Native split action choice failed");
    id split = find_label(view, @"Native split");
    check([split accessibilityPerformPress], "Native split primary action failed");
    auto frame = [split accessibilityFrame];
    check(frame.size.width > 0 && frame.size.height > 0, "Native accessibility frame is empty");
    [window setContentSize:NSMakeSize(500, 540)];
    [view displayIfNeeded];
    [window orderFrontRegardless];
}

void scars_after_delay(std::function<void()> callback) {
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 300 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
      callback();
    });
}

namespace {
NSWindow* developer_window() {
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Developer Tools"])
            return candidate;
    throw std::runtime_error("Missing developer tools window");
}
NSMenuItem* developer_shortcut(NSMenu* menu) {
    for (NSMenuItem* item in menu.itemArray) {
        if (item.tag == 2 && [item.keyEquivalent isEqualToString:@"i"])
            return item;
        if (item.submenu)
            if (auto* nested = developer_shortcut(item.submenu))
                return nested;
    }
    return nil;
}
} // namespace
void scars_inspect_developer_tools() {
    auto* panel = developer_window();
    check([panel isKindOfClass:NSPanel.class], "Developer tools should use a utility panel");
    check(panel.level == NSFloatingWindowLevel, "Developer tools are not floating above application windows");
    check((panel.styleMask & NSWindowStyleMaskMiniaturizable) != 0, "Developer tools cannot be minimized");
    check([panel standardWindowButton:NSWindowMiniaturizeButton].enabled,
          "Developer tools minimize button is disabled");
    check(((NSPanel*)panel).hidesOnDeactivate, "Developer tools should stay within their own application");
    id tree = find_label(panel.contentView, @"Element tree");
    check(tree && [[tree accessibilityRole] isEqualToString:NSAccessibilityOutlineRole], "Developer tools do not use the shared TreeView");
    auto* shortcut = developer_shortcut(NSApp.mainMenu);
    check(shortcut != nil &&
              shortcut.keyEquivalentModifierMask == (NSEventModifierFlagCommand | NSEventModifierFlagOption),
          "Developer tools shortcut is not Command-Option-I");
    id pick = find_label(panel.contentView, @"Pick an element in the application");
    check(pick != nil && [pick accessibilityPerformPress], "Developer tools Pick element action is missing");
    auto escape = [NSEvent keyEventWithType:NSEventTypeKeyDown
                                   location:NSZeroPoint
                              modifierFlags:0
                                  timestamp:0
                               windowNumber:panel.windowNumber
                                    context:nil
                                 characters:@"\x1b"
                charactersIgnoringModifiers:@"\x1b"
                                  isARepeat:NO
                                    keyCode:53];
    [panel.contentView keyDown:escape];
    check(![[pick accessibilityValue] boolValue], "Escape in the inspector did not cancel element picking");
    check([pick accessibilityPerformPress], "Could not re-enable picking after cancellation");
    NSWindow* source = nil;
    for (NSWindow* w in NSApp.windows)
        if ([w.title isEqualToString:@"SCARS Native Test"])
            source = w;
    [source makeKeyAndOrderFront:nil];
    check(panel.visible && panel.level > source.level,
          "Focusing an application window hid or lowered its developer tools");
}
void scars_minimize_developer_tools() {
    [developer_window() miniaturize:nil];
}
void scars_check_developer_tools_restored() {
    auto* panel = developer_window();
    check(!panel.miniaturized && panel.visible, "Developer tools were not restored after minimization");
    check(panel.level == NSFloatingWindowLevel, "Restoring developer tools lost their floating level");
}

void scars_report_native_visibility() {
    desktop_diagnostics();
    for (NSWindow* window in NSApp.windows)
        if ([window.title isEqualToString:@"SCARS Native Test"])
            NSLog(@"SCARS test visibility: active=%d visible=%d activeSpace=%d occlusion=%lu frame=%@",
                  NSApp.active, window.visible, window.onActiveSpace, (unsigned long)window.occlusionState,
                  NSStringFromRect(window.frame));
}

bool scars_native_window_on_active_space() {
    for (NSWindow* window in NSApp.windows)
        if ([window.title isEqualToString:@"SCARS Native Test"])
            return window.onActiveSpace;
    return false;
}

void scars_inspect_text_and_titlebar() {
    NSWindow* window = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"SCARS Native Test"]) {
            window = candidate;
            break;
        }
    check(window != nil, "Native text test window is missing");
    NSView<NSTextInputClient>* view = (NSView<NSTextInputClient>*)window.contentView;
    id text = find_label(view, @"Native text");
    check(text && [[text accessibilityRole] isEqualToString:NSAccessibilityTextFieldRole],
          "Text field accessibility role is missing");
    [text setAccessibilityFocused:YES];
    [view insertText:@"海🚀" replacementRange:NSMakeRange(NSNotFound, 0)];
    check([[text accessibilityValue] isEqualToString:@"海🚀"], "Native insertion failed");
    [view setMarkedText:@"に" selectedRange:NSMakeRange(1, 0) replacementRange:NSMakeRange(NSNotFound, 0)];
    check(view.hasMarkedText && view.markedRange.location == 3, "IME marked range must use UTF-16 offsets");
    [view insertText:@"日本" replacementRange:NSMakeRange(NSNotFound, 0)];
    check(!view.hasMarkedText && [[text accessibilityValue] isEqualToString:@"海🚀日本"],
          "Native IME commit failed");
    [view doCommandBySelector:NSSelectorFromString(@"moveLeftAndModifySelection:")];
    check(view.selectedRange.location == 4 && view.selectedRange.length == 1,
          "Native selection movement failed");
    [view doCommandBySelector:NSSelectorFromString(@"undo:")];
    check([[text accessibilityValue] isEqualToString:@"海🚀"], "Native undo failed");
    [view doCommandBySelector:NSSelectorFromString(@"redo:")];
    check([[text accessibilityValue] isEqualToString:@"海🚀日本"], "Native redo failed");
    NSRange actual;
    check([view firstRectForCharacterRange:view.selectedRange actualRange:&actual].size.height > 0,
          "IME candidate anchor is empty");
    id list = find_label(view, @"Native list");
    check(list && [[list accessibilityRole] isEqualToString:NSAccessibilityListRole],
          "Native list role is missing");
    id row = find_label(view, @"List beta");
    check(row && [row accessibilityPerformPress] && [row isAccessibilitySelected],
          "Native list row selection failed");
    check(window.titlebarAccessoryViewControllers.count == 1,
          "Theme toggle is not installed as a titlebar accessory");
    NSButton* toggle = nil;
    for (NSView* child in window.titlebarAccessoryViewControllers.firstObject.view.subviews)
        if ([child isKindOfClass:NSButton.class])
            toggle = (NSButton*)child;
    check(toggle && ![toggle isDescendantOf:window.contentView],
          "Theme toggle must remain outside the client area");
    auto check_alignment = [&] {
        [window.contentView.superview layoutSubtreeIfNeeded];
        const NSRect toggle_frame = [toggle convertRect:toggle.bounds toView:nil];
        NSButton* close = [window standardWindowButton:NSWindowCloseButton];
        const NSRect close_frame = [close convertRect:close.bounds toView:nil];
        check(std::abs(NSMidY(toggle_frame) - NSMidY(close_frame)) <= 1,
              "Theme toggle must be vertically centered with the native titlebar controls");
        check(std::abs(NSMidX(toggle.frame) - NSMidX(toggle.superview.bounds)) <= .5,
              "Theme toggle must be horizontally centered in its accessory");
    };
    check_alignment();
    const auto before = window.contentView.frame.size;
    [toggle performClick:nil];
    check(toggle.state == NSControlStateValueOn &&
              [window.appearance.name isEqualToString:NSAppearanceNameAqua],
          "Native light theme toggle failed");
    check_alignment();
    [toggle performClick:nil];
    check(toggle.state == NSControlStateValueOff &&
              [window.appearance.name isEqualToString:NSAppearanceNameDarkAqua],
          "Native dark theme toggle failed");
    check_alignment();
    check(NSEqualSizes(before, window.contentView.frame.size),
          "Titlebar toggle changed the client dimensions");
    id checkbox = find_label(view, @"Native checkbox");
    [checkbox setAccessibilityFocused:YES]; // Stop the field caret before static-frame settling checks.
}
