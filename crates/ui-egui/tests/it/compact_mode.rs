//! Compact phone layout (< 700 logical px, targeting 360×740) and desktop preservation.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::{COMPACT_TOUCH, COMPACT_WIDTH, PdfCraftApp, is_compact, is_compact_width};

fn fixture(n: usize) -> Vec<u8> {
    let mut objs: Vec<String> = vec!["<< /Type /Catalog /Pages 2 0 R >>".into()];
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} /MediaBox [0 0 200 300] >>", kids.join(" ")));
    objs.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into());
    for i in 0..n {
        objs.push(format!("<< /Type /Page /Parent 2 0 R /Contents {} 0 R /Resources << /Font << /F1 3 0 R >> >> >>", 5 + 2 * i));
        let body = format!("BT /F1 24 Tf 20 150 Td (Page {}) Tj ET", i + 1);
        objs.push(format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()));
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn harness_with_doc(size: egui::Vec2, name: &str) -> Harness<'static, PdfCraftApp> {
    let name = name.to_owned();
    let mut h = Harness::builder().with_size(size).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes(&name, None, fixture(2)).unwrap();
        app
    });
    h.run_steps(8);
    h
}

#[test]
fn compact_width_helper_switches_at_700() {
    assert!(is_compact_width(360.0));
    assert!(is_compact_width(COMPACT_WIDTH - 0.1));
    assert!(!is_compact_width(COMPACT_WIDTH));
    assert!(!is_compact_width(1400.0));

    let narrow = harness_with_doc(egui::vec2(360.0, 740.0), "a.pdf");
    assert!(is_compact(&narrow.ctx));

    let wide = harness_with_doc(egui::vec2(700.0, 740.0), "a.pdf");
    assert!(!is_compact(&wide.ctx));
}

#[test]
fn compact_layout_at_360x740_fits_chrome_and_hides_side_panels() {
    let mut h = harness_with_doc(egui::vec2(360.0, 740.0), "very-long-contract-filename-for-mobile-testing.pdf");
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 740.0));

    assert!(is_compact(&h.ctx));
    let spacing = &h.ctx.global_style().spacing;
    assert!(spacing.interact_size.y >= COMPACT_TOUCH, "touch height {}", spacing.interact_size.y);
    assert!(spacing.button_padding.y >= 10.0, "compact button_padding.y {}", spacing.button_padding.y);

    // Desktop tab strip is absent in compact mode.
    assert!(h.query_by_role_and_label(Role::Button, "Home").is_none());

    // Compact top bar: All tools, Save, More tools ("⋯"), all inside 360 px and >= 44 px tall.
    let menu = h.get_by_role_and_label(Role::Button, "All tools").rect();
    let save = h.get_by_label_contains("Save (").rect();
    let more = h.get_by_role_and_label(Role::Button, "More tools").rect();
    for (name, r) in [("all tools", menu), ("save", save), ("more", more)] {
        assert!(window.contains_rect(r), "{name} outside window: {r:?}");
        assert!(r.height() >= COMPACT_TOUCH - 0.5, "{name} touch height too small: {r:?}");
    }
    assert!(save.width() >= COMPACT_TOUCH - 0.5);
    assert!(more.width() >= COMPACT_TOUCH - 0.5);
    assert!(!menu.intersects(save) && !save.intersects(more) && !menu.intersects(more));

    // Side panels and right rail do not eat document width.
    let doc_w = h.state().views[0].viewport_rect().width();
    assert!(doc_w >= 359.0, "document viewport should span full width, got {doc_w}");

    // Bottom bar: the view controls in a single tight row.
    let out_btn = h.get_by_label_contains("Zoom out").rect();
    let next_btn = h.get_by_label("Next page").rect();
    assert!(out_btn.height() >= 28.0, "out_btn {out_btn:?}");
    assert!(next_btn.height() >= 28.0, "next_btn {next_btn:?}");
    assert!((out_btn.top() - next_btn.top()).abs() < 1.0, "same row: {out_btn:?} vs {next_btn:?}");
    assert!(out_btn.top() > 680.0, "bottom bar should sit at bottom: {out_btn:?}");

    // Overflow "⋯" menu exposes Find text and Edit page in image editor / Paint.
    h.get_by_role_and_label(Role::Button, "More tools").click();
    h.run_steps(4);
    assert!(h.query_by_label("Find text").is_some(), "Find text in overflow menu");
    let paint = if cfg!(windows) { "Edit page in Paint" } else { "Edit page in image editor" };
    assert!(h.query_by_label(paint).is_some(), "Paint button in overflow menu");
}

#[test]
fn desktop_layout_at_740x360_restores_desktop_chrome() {
    let h = harness_with_doc(egui::vec2(740.0, 360.0), "landscape.pdf");
    assert!(!is_compact(&h.ctx));
    assert!(h.ctx.global_style().spacing.interact_size.y < COMPACT_TOUCH);
    assert!(h.query_by_role_and_label(Role::Button, "Home").is_some(), "desktop tab strip present");
}
