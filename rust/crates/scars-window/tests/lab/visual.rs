use super::*;

fn pixels(h: &Host, r: &Renderer, size: (i32, i32), scale: i32) -> Vec<u8> {
    let mut surface = raster::render(h, r, size, scale).unwrap();
    let dimensions = (size.0 * scale, size.1 * scale);
    let mut bytes = vec![0; (dimensions.0 * dimensions.1 * 4) as usize];
    assert!(surface.read_pixels(
        &skia::ImageInfo::new(
            dimensions,
            skia::ColorType::RGBA8888,
            skia::AlphaType::Unpremul,
            None
        ),
        &mut bytes,
        dimensions.0 as usize * 4,
        (0, 0),
    ));
    bytes
}

#[test]
fn unicode_state_matrix_preserves_editing_hit_testing_and_scaled_field_pixels() {
    let r = Renderer::default();
    for profile in ["long", "rtl", "emoji"] {
        for state in ["default", "disabled", "readonly", "error"] {
            for (theme, palette) in [("dark", Theme::default()), ("light", Theme::light())] {
                for size in [(440, 740), (1120, 900)] {
                    let lab = Lab::new().unwrap();
                    lab.text_profile(profile).unwrap();
                    lab.preview_state(state).unwrap();
                    lab.pages.select("fields");
                    let h = mounted(&lab);
                    h.set_theme(palette);
                    let viewport = Size::new(size.0 as f64, size.1 as f64);
                    layout(&h, viewport, &r);
                    let name = find(&h, "name");
                    h.reveal_rect(name.id, name.bounds);
                    layout(&h, viewport, &r);
                    let name = find(&h, "name");
                    let original = lab.text.get();
                    if state == "disabled" {
                        assert!(!h.focus(name.id).unwrap());
                        assert!(!h.set_text(name.id, "Changed").unwrap());
                    } else {
                        assert!(h.focus(name.id).unwrap());
                        assert!(h.select_text(name.id, Selection::default()).unwrap());
                        if state == "readonly" {
                            assert!(!h.set_text(name.id, "Changed").unwrap());
                            assert_eq!(lab.text.get(), original);
                        } else {
                            assert!(h.set_text(name.id, "Changed").unwrap());
                            h.set_text(name.id, &original).unwrap();
                        }
                    }
                    assert_eq!(lab.text.get(), original);
                    layout(&h, viewport, &r);
                    let name = find(&h, "name");
                    if state == "error" {
                        assert_eq!(
                            name.validation.as_ref().unwrap().status,
                            ValidationStatus::Incomplete
                        );
                    }
                    let point = Point {
                        x: name.bounds.x + name.bounds.width / 2.,
                        y: name.bounds.y + name.bounds.height / 2.,
                    };
                    let index = h.text_index(name.id, point);
                    assert!(text::grapheme_boundaries(&original).contains(&index));
                    for scale in [1, 2] {
                        // Input remains logical even when the framebuffer has twice as many pixels.
                        let physical = Point {
                            x: point.x * scale as f64,
                            y: point.y * scale as f64,
                        };
                        let logical = Point {
                            x: physical.x / scale as f64,
                            y: physical.y / scale as f64,
                        };
                        assert_eq!(
                            h.hit_test(logical).unwrap(),
                            (state != "disabled").then_some(name.id)
                        );
                        let bytes = pixels(&h, &r, size, scale);
                        let x = ((name.bounds.x + 5.) * scale as f64) as usize;
                        let y = ((name.bounds.y + name.bounds.height / 2.) * scale as f64) as usize;
                        let offset = (y * (size.0 * scale) as usize + x) * 4;
                        let [a, red, green, blue] = palette.field.to_be_bytes();
                        assert_eq!(
                            &bytes[offset..offset + 4],
                            &[red, green, blue, a],
                            "{profile}/{state}/{theme}/{}@{scale}x",
                            size.0
                        );
                    }
                    paint(
                        &h,
                        size,
                        &r,
                        &format!("text-{profile}-{state}-{theme}-{}", size.0),
                    );
                }
            }
        }
    }
}

#[test]
fn focus_ring_changes_pixels_only_inside_the_focused_field_at_both_scales() {
    let r = Renderer::default();
    let lab = Lab::new().unwrap();
    lab.pages.select("fields");
    let h = mounted(&lab);
    let size = (440, 740);
    let viewport = Size::new(440., 740.);
    layout(&h, viewport, &r);
    let field = find(&h, "name");
    h.focus(field.id).unwrap();
    for scale in [1, 2] {
        h.set_active(false);
        layout(&h, viewport, &r);
        let before = pixels(&h, &r, size, scale);
        h.set_active(true);
        layout(&h, viewport, &r);
        let after = pixels(&h, &r, size, scale);
        let clip = find(&h, "name").clip;
        let mut changed = 0;
        for (index, (a, b)) in before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(after.as_chunks::<4>().0)
            .enumerate()
        {
            if a != b {
                let width = (size.0 * scale) as usize;
                let point = Point {
                    x: (index % width) as f64 / scale as f64,
                    y: (index / width) as f64 / scale as f64,
                };
                assert!(
                    clip.contains(point),
                    "focus escaped field clip at {point:?}"
                );
                changed += 1;
            }
        }
        assert!(changed > 0, "focus must change actual pixels at {scale}x");
    }
}
