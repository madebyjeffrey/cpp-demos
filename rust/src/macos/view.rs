use super::*;
use objc2::runtime::Sel;

pub(super) struct ViewIvars {
    state: RefCell<Weak<SurfaceState>>,
    tracking: RefCell<Option<Retained<NSTrackingArea>>>,
}
define_class!(
    // SAFETY: NSView supports subclassing. Designated initialization is done in new().
    // Event and NSTextInputClient signatures follow the generated AppKit declarations.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ViewIvars]
    pub(super) struct ContentView;
    unsafe impl NSObjectProtocol for ContentView {}
    impl ContentView {
        #[unsafe(method_id(scarsAccessibilityEvents))]
        fn accessibility_events(&self)->Retained<NSArray<NSString>>{components::post_events()}
        #[unsafe(method(scarsClearAccessibilityEvents))]
        fn clear_accessibility_events(&self){components::clear_events()}
        #[unsafe(method(scarsAccessibilitySnapshotCount))]
        fn accessibility_snapshot_count(&self)->usize{with_surface(&self.weak(),|s|components::snapshot_count(s)).unwrap_or(0)}
        #[unsafe(method(scarsAccessibilityPostCount))]
        fn accessibility_posts(&self)->u64{components::post_count()}

        #[unsafe(method_id(accessibilityChildren))]
        fn component_children(&self)->Option<Retained<NSArray<AnyObject>>> {with_surface(&self.weak(),components::children).flatten()}
        #[unsafe(method_id(accessibilityFocusedUIElement))]
        fn component_focus(&self)->Option<Retained<AnyObject>> {with_surface(&self.weak(),components::focused).flatten()}
        #[unsafe(method_id(accessibilityHitTest:))]
        fn component_hit(&self,p:NSPoint)->Option<Retained<AnyObject>> {with_surface(&self.weak(),|s|components::hit_test(s,p)).flatten()}
        #[unsafe(method(isAccessibilityElement))]
        fn component_element(&self)->bool {false}
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking(&self) {
            // SAFETY: Super implementation and tracking area ownership stay on main thread.
            unsafe { let _: () = msg_send![super(self), updateTrackingAreas]; }
            if let Some(old) = self.ivars().tracking.borrow_mut().take() { self.removeTrackingArea(&old); }
            let options = NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect;
            // SAFETY: owner is this retained view; AppKit removes its tracking area on destruction.
            let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(),self.bounds(),options,Some(self),None) };
            self.addTrackingArea(&area); *self.ivars().tracking.borrow_mut() = Some(area);
        }
        #[unsafe(method(resetCursorRects))]
        fn reset_cursors(&self) {
            with_surface(&self.weak(),|s| {
                let cursor = match s.cursor.get() {
                    Cursor::Arrow => NSCursor::arrowCursor(), Cursor::Text => NSCursor::IBeamCursor(),
                    Cursor::ResizeHorizontal => NSCursor::columnResizeCursor(), Cursor::ResizeVertical => NSCursor::rowResizeCursor(),
                };
                self.addCursorRect_cursor(self.bounds(),&cursor);
            });
        }
        #[unsafe(method(tick:))]
        fn tick(&self, _: &CADisplayLink) { with_surface(&self.weak(),|s| { render(s,false); refresh_motion(s); }); }
        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            // SAFETY: The superclass implements this NSView notification hook.
            unsafe { let _: () = msg_send![super(self), viewDidChangeEffectiveAppearance]; }
            with_surface(&self.weak(),|s| { emit(s,Event::AppearanceChanged(effective_appearance(s))); s.dirty.set(true); update_pause(s); });
        }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) { self.pointer(event,PointerKind::Down); }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) { self.pointer(event,PointerKind::Up); }
        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self, event: &NSEvent) { self.pointer(event,PointerKind::Down); }
        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self, event: &NSEvent) { self.pointer(event,PointerKind::Up); }
        #[unsafe(method(otherMouseDown:))]
        fn other_down(&self, event: &NSEvent) { self.pointer(event,PointerKind::Down); }
        #[unsafe(method(otherMouseUp:))]
        fn other_up(&self, event: &NSEvent) { self.pointer(event,PointerKind::Up); }
        #[unsafe(method(mouseMoved:))]
        fn mouse_move(&self, event: &NSEvent) { self.pointer(event,PointerKind::Move); }
        #[unsafe(method(mouseDragged:))]
        fn mouse_drag(&self, event: &NSEvent) { self.pointer(event,PointerKind::Move); }
        #[unsafe(method(rightMouseDragged:))]
        fn right_drag(&self, event: &NSEvent) { self.pointer(event,PointerKind::Move); }
        #[unsafe(method(otherMouseDragged:))]
        fn other_drag(&self, event: &NSEvent) { self.pointer(event,PointerKind::Move); }
        #[unsafe(method(mouseEntered:))]
        fn mouse_enter(&self, event: &NSEvent) { self.pointer(event,PointerKind::Enter); }
        #[unsafe(method(mouseExited:))]
        fn mouse_leave(&self, event: &NSEvent) { self.pointer(event,PointerKind::Leave); }
        #[unsafe(method(scrollWheel:))]
        fn scroll(&self, event: &NSEvent) { self.pointer(event,PointerKind::Scroll); }
        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            with_surface(&self.weak(),|s| {
                let key = key_event(event,true);
                let client = s.text_input.borrow().clone();
                if let Some(client) = client {
                    if client.intercept_key(&key) { return; }
                    if client.state().is_some_and(|state| state.active && state.editable)
                        && !(key.key == Key::Enter && key.modifiers.command) {
                        // SAFETY: Event is from this view's main-thread responder chain.
                        self.interpretKeyEvents(&NSArray::from_slice(&[event]));
                        sync_secure_input(s,false); return;
                    }
                }
                emit(s,Event::Key(key));
            });
        }
        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) { with_surface(&self.weak(),|s| emit(s,Event::Key(key_event(event,false)))); }
        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) { with_surface(&self.weak(),|s| emit(s,Event::Modifiers(modifiers(event.modifierFlags())))); }
        #[unsafe(method(cut:))]
        fn cut(&self, _: Option<&AnyObject>) { self.edit_command("cut:"); }
        #[unsafe(method(copy:))]
        fn copy(&self, _: Option<&AnyObject>) { self.edit_command("copy:"); }
        #[unsafe(method(paste:))]
        fn paste(&self, _: Option<&AnyObject>) { self.edit_command("paste:"); }
        #[unsafe(method(selectAll:))]
        fn select_all(&self, _: Option<&AnyObject>) { self.edit_command("selectAll:"); }
        #[unsafe(method(undo:))]
        fn undo(&self, _: Option<&AnyObject>) { self.edit_command("undo:"); }
        #[unsafe(method(redo:))]
        fn redo(&self, _: Option<&AnyObject>) { self.edit_command("redo:"); }
    }
    unsafe impl NSUserInterfaceValidations for ContentView {
        #[unsafe(method(validateUserInterfaceItem:))]
        fn validate(&self, item: &ProtocolObject<dyn NSValidatedUserInterfaceItem>) -> bool {
            with_surface(&self.weak(),|s| {
                if !s.open.get() || s.native.attachedSheet().is_some() { return false; }
                let Some(state) = text::snapshot(s) else { return false; };
                let Some(action) = item.action() else { return false; };
                match action.name().to_bytes() {
                    b"undo:" => state.editable && state.can_undo, b"redo:" => state.editable && state.can_redo,
                    b"cut:" => state.editable && !state.secure && state.selection.length > 0,
                    b"copy:" => !state.secure && state.selection.length > 0,
                    b"paste:" => state.editable, b"selectAll:" => !state.text.is_empty(), _ => false,
                }
            }).unwrap_or(false)
        }
    }
    unsafe impl NSTextInputClient for ContentView {
        #[unsafe(method(insertText:replacementRange:))]
        fn insert_text(&self, value: &AnyObject, range: NSRange) {
            with_surface(&self.weak(),|s| { if let Some(client) = text::client(s) { client.insert(text::string(value),text::range(range)); sync_secure_input(s,false); } });
        }
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn mark_text(&self, value: &AnyObject, selection: NSRange, replacement: NSRange) {
            with_surface(&self.weak(),|s| { if let Some(client) = text::client(s) { client.mark(text::string(value),text::range(selection).unwrap_or_default(),text::range(replacement)); } });
        }
        #[unsafe(method(unmarkText))]
        fn unmark_text(&self) { with_surface(&self.weak(),|s| { if let Some(client) = text::client(s) { client.unmark(true); } }); }
        #[unsafe(method(doCommandBySelector:))]
        fn do_command(&self, selector: Sel) { with_surface(&self.weak(),|s| { if let Some(client) = text::client(s) { client.command(&selector.name().to_string_lossy()); } }); }
        #[unsafe(method(selectedRange))]
        fn selected_range(&self) -> NSRange { with_surface(&self.weak(),|s| text::snapshot(s).map(|st| text::native_range(st.selection)).unwrap_or_else(text::missing)).unwrap_or_else(text::missing) }
        #[unsafe(method(markedRange))]
        fn marked_range(&self) -> NSRange { with_surface(&self.weak(),|s| text::snapshot(s).and_then(|st| st.marked).map(text::native_range).unwrap_or_else(text::missing)).unwrap_or_else(text::missing) }
        #[unsafe(method(hasMarkedText))]
        fn has_marked(&self) -> bool { with_surface(&self.weak(),|s| text::snapshot(s).is_some_and(|st| st.marked.is_some())).unwrap_or(false) }
        #[unsafe(method_id(validAttributesForMarkedText))]
        fn valid_attributes(&self) -> Retained<NSArray<NSString>> { NSArray::new() }
        #[unsafe(method_id(attributedSubstringForProposedRange:actualRange:))]
        fn substring(&self, range: NSRange, actual: *mut NSRange) -> Option<Retained<NSAttributedString>> {
            with_surface(&self.weak(),|s| {
                let st = text::snapshot(s)?;
                if st.secure { return None; }
                let (value,r) = text::substring(&st.text,range)?;
                // SAFETY: NSTextInputClient guarantees a valid output pointer or null.
                unsafe { if !actual.is_null() { *actual = r; } }
                Some(NSAttributedString::initWithString(NSAttributedString::alloc(),&ns(&value)))
            }).flatten()
        }
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn first_rect(&self, range: NSRange, actual: *mut NSRange) -> NSRect {
            with_surface(&self.weak(),|s| {
                let Some(client) = text::client(s) else { return NSRect::ZERO; };
                let r = client.rectangle(text::range(range).unwrap_or_default());
                // SAFETY: NSTextInputClient supplies a valid output pointer or null.
                unsafe { if !actual.is_null() { *actual = range; } }
                let local = NSRect::new(NSPoint::new(r.origin.x,r.origin.y),ns_size(r.size));
                s.native.convertRectToScreen(self.convertRect_toView(local,None))
            }).unwrap_or(NSRect::ZERO)
        }
        #[unsafe(method(characterIndexForPoint:))]
        fn character_index(&self, point: NSPoint) -> usize {
            with_surface(&self.weak(),|s| {
                let Some(client) = text::client(s) else { return NSNotFound as usize; };
                let rect = s.native.convertRectFromScreen(NSRect::new(point,NSSize::ZERO));
                let local = self.convertPoint_fromView(rect.origin,None);
                client.index(Point::new(local.x,local.y))
            }).unwrap_or(NSNotFound as usize)
        }
    }
);

impl ContentView {
    pub(super) fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(ViewIvars {
            state: RefCell::new(Weak::new()),
            tracking: RefCell::new(None),
        });
        // SAFETY: NSView's designated initializer; ivars are initialized first.
        unsafe { msg_send![super(allocated), initWithFrame: frame] }
    }
    pub(super) fn connect(&self, weak: Weak<SurfaceState>) {
        *self.ivars().state.borrow_mut() = weak;
    }
    fn weak(&self) -> Weak<SurfaceState> {
        self.ivars().state.borrow().clone()
    }
    fn edit_command(&self, selector: &str) {
        with_surface(&self.weak(), |s| {
            if !s.open.get() || s.native.attachedSheet().is_some() {
                return;
            }
            if let Some(client) = text::client(s) {
                client.command(selector);
                sync_secure_input(s, false);
            }
        });
    }
    fn pointer(&self, event: &NSEvent, kind: PointerKind) {
        with_surface(&self.weak(), |s| {
            let p = self.convertPoint_fromView(event.locationInWindow(), None);
            let button_event = matches!(kind, PointerKind::Down | PointerKind::Up)
                || matches!(
                    event.r#type(),
                    NSEventType::LeftMouseDragged
                        | NSEventType::RightMouseDragged
                        | NSEventType::OtherMouseDragged
                );
            if kind == PointerKind::Down {
                s.native.makeFirstResponder(Some(self));
            }
            emit(
                s,
                Event::Pointer(PointerEvent {
                    kind,
                    position: Point::new(p.x, p.y),
                    button: if button_event {
                        event.buttonNumber() as i32
                    } else {
                        0
                    },
                    scroll_x: if kind == PointerKind::Scroll {
                        event.scrollingDeltaX()
                    } else {
                        0.
                    },
                    scroll_y: if kind == PointerKind::Scroll {
                        event.scrollingDeltaY()
                    } else {
                        0.
                    },
                    modifiers: modifiers(event.modifierFlags()),
                    click_count: if button_event {
                        event.clickCount().max(1) as u32
                    } else {
                        0
                    },
                }),
            );
            if let Some(context) = s.view.inputContext() {
                context.invalidateCharacterCoordinates();
            }
        });
    }
}
pub(super) fn modifiers(f: NSEventModifierFlags) -> Modifiers {
    Modifiers {
        shift: f.contains(NSEventModifierFlags::Shift),
        control: f.contains(NSEventModifierFlags::Control),
        alt: f.contains(NSEventModifierFlags::Option),
        command: f.contains(NSEventModifierFlags::Command),
    }
}
pub(super) fn flags(m: Modifiers) -> NSEventModifierFlags {
    let mut f = NSEventModifierFlags::empty();
    f.set(NSEventModifierFlags::Shift, m.shift);
    f.set(NSEventModifierFlags::Control, m.control);
    f.set(NSEventModifierFlags::Option, m.alt);
    f.set(NSEventModifierFlags::Command, m.command);
    f
}
fn key_event(e: &NSEvent, down: bool) -> KeyEvent {
    let m = modifiers(e.modifierFlags());
    let text = e.characters().map(|s| s.to_string()).unwrap_or_default();
    KeyEvent {
        key: Key::from_native(
            e.keyCode(),
            &e.charactersIgnoringModifiers()
                .map(|s| s.to_string())
                .unwrap_or_default(),
        ),
        down,
        repeat: e.isARepeat(),
        modifiers: modifiers(e.modifierFlags()),
        native_code: e.keyCode(),
        text: if down
            && !m.command
            && !m.control
            && !text.chars().next().is_some_and(char::is_control)
        {
            text
        } else {
            String::new()
        },
    }
}
pub(super) fn refresh_motion(s: &Rc<SurfaceState>) {
    let reduced = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion();
    if s.reduced_motion.replace(reduced) != reduced {
        emit(s, Event::ReducedMotionChanged(reduced));
    }
}
