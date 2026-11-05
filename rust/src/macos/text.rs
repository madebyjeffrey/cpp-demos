use super::*;

// Carbon's secure input API is process-wide and nestable; each surface balances
// exactly one successful enable with one disable on blur, hide, replacement or close.
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn EnableSecureEventInput() -> i32;
    fn DisableSecureEventInput() -> i32;
}
pub(super) fn sync_secure_input(s: &Rc<SurfaceState>, force_off: bool) {
    let desired = !force_off
        && s.open.get()
        && s.native.isKeyWindow()
        && s.native.isVisible()
        && s.app.upgrade().is_some_and(|a| a.native.isActive())
        && snapshot(s).is_some_and(|state| state.active && state.editable && state.secure);
    if desired == s.secure_input.get() {
        return;
    }
    // SAFETY: Main-thread process-level secure input nesting is tracked per surface.
    let status = unsafe {
        if desired {
            EnableSecureEventInput()
        } else {
            DisableSecureEventInput()
        }
    };
    if status == 0 {
        s.secure_input.set(desired);
    } else if let Some(app) = s.app.upgrade() {
        report(&app, error("Cannot change macOS secure input state"));
    }
}
pub(super) fn deactivate_input(s: &Rc<SurfaceState>) {
    sync_secure_input(s, true);
    let client = s.text_input.borrow().clone();
    if let (Some(client), Some(app)) = (client, s.app.upgrade()) {
        guarded(&app, || client.deactivate());
    }
}
pub(super) fn client(s: &SurfaceState) -> Option<Rc<dyn TextInputClient>> {
    s.text_input.borrow().clone()
}
pub(super) fn snapshot(s: &SurfaceState) -> Option<TextInputState> {
    client(s)?.state().filter(|st| st.active)
}
pub(super) fn missing() -> NSRange {
    NSRange::new(NSNotFound as usize, 0)
}
pub(super) fn range(r: NSRange) -> Option<TextRange> {
    (r.location != NSNotFound as usize).then_some(TextRange {
        start: r.location,
        length: r.length,
    })
}
pub(super) fn native_range(r: TextRange) -> NSRange {
    NSRange::new(r.start, r.length)
}
pub(super) fn string(o: &AnyObject) -> String {
    if let Some(s) = o.downcast_ref::<NSString>() {
        s.to_string()
    } else if let Some(s) = o.downcast_ref::<NSAttributedString>() {
        s.string().to_string()
    } else {
        String::new()
    }
}
pub(super) fn substring(text: &str, r: NSRange) -> Option<(String, NSRange)> {
    if r.location == NSNotFound as usize {
        return None;
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    if r.location > units.len() {
        return None;
    }
    let mut start = r.location;
    let mut end = start.saturating_add(r.length).min(units.len());
    // Expand an AppKit range that bisects a surrogate pair, returning the actual range.
    if start > 0 && start < units.len() && (0xdc00..=0xdfff).contains(&units[start]) {
        start -= 1;
    }
    if end > 0 && end < units.len() && (0xdc00..=0xdfff).contains(&units[end]) {
        end += 1;
    }
    Some((
        String::from_utf16(&units[start..end]).ok()?,
        NSRange::new(start, end - start),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ime_ranges_preserve_surrogate_pairs_and_bound_extreme_ranges() {
        let (s, actual) = substring("A😀B", NSRange::new(2, 1)).unwrap();
        assert_eq!(s, "😀");
        assert_eq!(actual, NSRange::new(1, 2));
        assert_eq!(
            substring("A😀B", NSRange::new(1, usize::MAX)).unwrap().0,
            "😀B"
        );
        assert!(substring("x", missing()).is_none());
        assert!(substring("x", NSRange::new(2, 0)).is_none());
    }
}
