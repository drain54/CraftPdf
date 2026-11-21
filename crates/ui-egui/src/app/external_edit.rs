//! Edit a page picture in another app (Paint on Windows). The picture goes to a temporary PNG,
//! upright as the page shows it, and opens there; every time that app saves the file, the new
//! version takes the picture's place on the page, one undo step per save, while the app stays
//! open. `CRAFTPDF_IMAGE_EDITOR` names another program to use instead.

#[cfg(not(target_arch = "wasm32"))]
mod pixels;

use super::*;

/// The picture's menu entry.
pub(crate) fn editor_label() -> &'static str {
    if cfg!(windows) { tl!("Edit in Paint") } else { tl!("Edit in image editor") }
}

/// The page's menu entry.
pub(crate) fn page_editor_label() -> &'static str {
    if cfg!(windows) { tl!("Edit page in Paint") } else { tl!("Edit page in image editor") }
}

/// Pictures out for editing, and when their files are next looked at.
#[derive(Default)]
pub(crate) struct ExternalEdits {
    #[cfg(not(target_arch = "wasm32"))]
    open: Vec<ExternalEdit>,
    #[cfg(not(target_arch = "wasm32"))]
    next_look: Option<std::time::Instant>,
}

#[cfg(not(target_arch = "wasm32"))]
type Stamp = (std::time::SystemTime, u64);

#[cfg(not(target_arch = "wasm32"))]
struct ExternalEdit {
    doc: DocId,
    page: usize,
    index: usize,
    path: std::path::PathBuf,
    /// Quarter turns clockwise the file was given to show upright; undone on the way back.
    turns: u8,
    /// The whole page went out, rendered at this many pixels per point (`index` unused): what
    /// comes back replaces the page.
    whole_page: Option<f32>,
    /// The file's version last put on the page (or written by us).
    seen: Option<Stamp>,
    /// A newer version, taken once it holds still for one more look: the other app may still be
    /// writing it.
    settling: Option<Stamp>,
}

#[cfg(not(target_arch = "wasm32"))]
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_millis(400);

#[cfg(not(target_arch = "wasm32"))]
fn stamp(path: &std::path::Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Open `path` in the image editor.
#[cfg(not(target_arch = "wasm32"))]
fn launch(path: &std::path::Path) -> std::io::Result<()> {
    match std::env::var_os("CRAFTPDF_IMAGE_EDITOR").filter(|e| !e.is_empty()) {
        Some(editor) => std::process::Command::new(editor).arg(path).spawn().map(drop),
        None => launch_default(path),
    }
}

/// Paint.
#[cfg(windows)]
fn launch_default(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match Command::new("mspaint.exe").arg(path).spawn() {
        Ok(_) => Ok(()),
        // Paint from the Store, reached only through the shell's app alias.
        Err(_) => Command::new("cmd.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .raw_arg(format!("/C start \"\" mspaint \"{}\"", path.display()))
            .spawn()
            .map(drop),
    }
}

/// The system's app for PNG files (Preview).
#[cfg(target_os = "macos")]
fn launch_default(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("open").arg(path).spawn().map(drop)
}

/// The desktop's app for PNG files.
#[cfg(all(unix, not(target_os = "macos"), not(target_arch = "wasm32")))]
fn launch_default(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(path).spawn().map(drop)
}

/// `rgba` as a PNG with no more channels than it uses.
#[cfg(not(target_arch = "wasm32"))]
fn png_of(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    use craft_image_codecs::{ChannelLayout as L, EncodeOptions, Format, Image};
    let (layout, data) = match pixels::pack(rgba) {
        pixels::Packed::Gray(d) => (L::Gray, d),
        pixels::Packed::Rgb(d) => (L::Rgb, d),
        pixels::Packed::Rgba(d) => (L::Rgba, d),
    };
    let img = Image::from_u8(w, h, layout, data).map_err(|e| e.to_string())?;
    craft_image_codecs::encode(&img, Format::Png, &EncodeOptions::default()).map_err(|e| e.to_string())
}

/// A file name that is safe on every system and says where the picture came from.
#[cfg(not(target_arch = "wasm32"))]
fn file_name(doc: &str, page: usize, index: usize) -> String {
    let stem = doc.rsplit_once('.').map_or(doc, |(s, _)| s);
    let clean: String = stem.chars().map(|c| if c.is_alphanumeric() || " -_()".contains(c) { c } else { '_' }).take(60).collect();
    let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() % 1_000_000);
    if index == usize::MAX {
        return format!("{} - page {} ({unique}).png", clean.trim(), page + 1);
    }
    format!("{} - page {} picture {} ({unique}).png", clean.trim(), page + 1, index + 1)
}

impl PdfCraftApp {
    /// Page right-click ▸ Edit page in Paint, and the mode bar's button. A plain scan (no text,
    /// one picture covering most of the page) sends its picture, which keeps the scan's own
    /// pixels; any other page (text, OCR'd and edited text included) goes as one whole picture.
    pub(crate) fn edit_page_externally(&mut self, page: usize) {
        let Some((_, id)) = self.active_ids() else { return };
        let Some(d) = self.session.get(id) else { return };
        let area = |r: [f64; 4]| ((r[2] - r[0]) * (r[3] - r[1])).abs();
        let page_area = d.info.pages.get(page).map_or(0.0, |p| f64::from(p.width) * f64::from(p.height));
        let largest = d
            .page_images(page)
            .iter()
            .enumerate()
            .filter(|(_, img)| !img.is_form && img.width > 0 && img.height > 0)
            .map(|(i, img)| (i, area(img.visible_rect.unwrap_or(img.rect))))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let plain_scan = d.text_lines(page).is_empty() && largest.is_some_and(|(_, a)| page_area > 0.0 && a >= page_area * 0.5);
        match largest {
            Some((index, _)) if plain_scan => self.edit_page_image_externally(page, index),
            _ => self.edit_whole_page_externally(page),
        }
    }

    /// The whole page, as one picture, in the image editor.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn edit_whole_page_externally(&mut self, _page: usize) {
        self.notify_tr("Editing pictures in another app needs the desktop version");
    }

    /// The whole page, as one picture, in the image editor: rendered as shown (comments left
    /// out: they stay on the page), and each save there replaces the page.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn edit_whole_page_externally(&mut self, page: usize) {
        let Some((_, id)) = self.active_ids() else { return };
        if let Some(open) = self.external_edits.open.iter().find(|e| e.doc == id && e.page == page && e.whole_page.is_some()) {
            let path = open.path.clone();
            if let Err(e) = launch(&path) {
                self.notify_fmt("Couldn't open the image editor: {e}", &[("e", &e.to_string())]);
            }
            return;
        }
        let prepared = (|| -> Result<(std::path::PathBuf, f32), String> {
            let doc = self.session.get(id).ok_or_else(|| tl!("The document is no longer open").to_string())?;
            if !doc.allows_modification() {
                return Err(tl!("The document's security settings don't allow this change").to_string());
            }
            let p = doc.info.pages.get(page).ok_or_else(|| tl!("The page is no longer in the document").to_string())?;
            let side = (f64::from(p.width).max(f64::from(p.height))) as f32;
            // 200 dpi, and no side past 6000 pixels.
            let scale = (200.0f32 / 72.0).min(6000.0 / side.max(1.0));
            let config = pdfcraft_render::RenderConfig {
                password: doc.password.as_deref().map(std::sync::Arc::from),
                hide_comments: true,
                ..Default::default()
            };
            let out = pdfcraft_render::PageRenderer::new(doc.bytes.clone(), config).render(pdfcraft_render::RenderRequest {
                page,
                scale,
                ..Default::default()
            });
            if let Some(e) = out.error {
                return Err(e);
            }
            let png = png_of(out.width, out.height, &out.rgba)?;
            let dir = std::env::temp_dir().join("PeDeEp edits");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(file_name(&doc.name, page, usize::MAX));
            std::fs::write(&path, png).map_err(|e| e.to_string())?;
            Ok((path, scale))
        })();
        let (path, scale) = match prepared {
            Ok(p) => p,
            Err(e) => {
                self.notify_fmt("Couldn't edit the page: {e}", &[("e", &e)]);
                return;
            }
        };
        if let Err(e) = launch(&path) {
            let _ = std::fs::remove_file(&path);
            self.notify_fmt("Couldn't open the image editor: {e}", &[("e", &e.to_string())]);
            return;
        }
        let seen = stamp(&path);
        self.external_edits.open.push(ExternalEdit {
            doc: id,
            page,
            index: usize::MAX,
            path,
            turns: 0,
            whole_page: Some(scale),
            seen,
            settling: None,
        });
        self.notify_fmt("Each save in the image editor replaces page {p} here", &[("p", &(page + 1).to_string())]);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint_after(LOOK_EVERY);
        }
    }

    /// Edit in Paint: picture `index` on `page` of the active document.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn edit_page_image_externally(&mut self, _page: usize, _index: usize) {
        self.notify_tr("Editing pictures in another app needs the desktop version");
    }

    /// Edit in Paint: picture `index` on `page` of the active document.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn edit_page_image_externally(&mut self, page: usize, index: usize) {
        let Some((view, id)) = self.active_ids() else { return };
        // Already out for editing: bring its file up again rather than start a second copy.
        if let Some(open) = self.external_edits.open.iter().find(|e| e.doc == id && e.page == page && e.whole_page.is_none() && e.index == index) {
            let path = open.path.clone();
            if let Err(e) = launch(&path) {
                self.notify_fmt("Couldn't open the image editor: {e}", &[("e", &e.to_string())]);
            }
            return;
        }
        let view_turn = self.views.get(view).map_or(0, |v| v.rotation % 360);
        let prepared = (|| -> Result<(std::path::PathBuf, u8), String> {
            let doc = self.session.get(id).ok_or_else(|| tl!("The document is no longer open").to_string())?;
            if !doc.allows_modification() {
                return Err(tl!("The document's security settings don't allow this change").to_string());
            }
            let img = doc.page_images(page).into_iter().nth(index).ok_or_else(|| tl!("The picture is no longer on the page").to_string())?;
            if img.is_form {
                return Err(tl!("Grouped artwork can't be edited as a picture").to_string());
            }
            let page_turn = doc.info.pages.get(page).map_or(0, |p| p.rotation % 360);
            let turns = pixels::upright_turns(img.matrix, (view_turn + page_turn) % 360);
            let (w, h, rgba) = doc.page_image_rgba(page, index)?;
            let (w, h, rgba) = pixels::turn_rgba(w, h, &rgba, turns).ok_or_else(|| tl!("The picture's pixels couldn't be read").to_string())?;
            let png = png_of(w, h, &rgba)?;
            let dir = std::env::temp_dir().join("PeDeEp edits");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(file_name(&doc.name, page, index));
            std::fs::write(&path, png).map_err(|e| e.to_string())?;
            Ok((path, turns))
        })();
        let (path, turns) = match prepared {
            Ok(p) => p,
            Err(e) => {
                self.notify_fmt("Couldn't edit the picture: {e}", &[("e", &e)]);
                return;
            }
        };
        if let Err(e) = launch(&path) {
            let _ = std::fs::remove_file(&path);
            self.notify_fmt("Couldn't open the image editor: {e}", &[("e", &e.to_string())]);
            return;
        }
        let seen = stamp(&path);
        self.external_edits.open.push(ExternalEdit { doc: id, page, index, path, turns, whole_page: None, seen, settling: None });
        self.notify_fmt("Each save in the image editor updates page {p} here", &[("p", &(page + 1).to_string())]);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint_after(LOOK_EVERY);
        }
    }

    /// Every frame: put each newly saved version of a picture out for editing in its place.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn poll_external_edits(&mut self) {}

    /// Every frame: put each newly saved version of a picture out for editing in its place.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn poll_external_edits(&mut self) {
        if self.external_edits.open.is_empty() {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint_after(LOOK_EVERY);
        }
        if self.external_edits.next_look.is_some_and(|t| now < t) {
            return;
        }
        self.external_edits.next_look = Some(now + LOOK_EVERY);
        // Closed documents take their files with them.
        let views: Vec<DocId> = self.views.iter().map(|v| v.id).collect();
        self.external_edits.open.retain(|e| {
            let open = views.contains(&e.doc);
            if !open {
                let _ = std::fs::remove_file(&e.path);
            }
            open
        });
        // A dialog over the document waits for its answer first; the change stays settling.
        let wait = self.modal_open();
        let mut ready = Vec::new();
        for (i, e) in self.external_edits.open.iter_mut().enumerate() {
            let Some(on_disk) = stamp(&e.path) else { continue };
            if Some(on_disk) == e.seen {
                e.settling = None;
            } else if e.settling == Some(on_disk) {
                if !wait {
                    ready.push(i);
                }
            } else {
                e.settling = Some(on_disk);
            }
        }
        for i in ready {
            self.take_external_version(i);
        }
    }

    /// Put the settled version of edit `i` on its page.
    #[cfg(not(target_arch = "wasm32"))]
    fn take_external_version(&mut self, i: usize) {
        let Some(e) = self.external_edits.open.get_mut(i) else { return };
        let Some(version) = e.settling.take() else { return };
        // Taken or refused, this version is dealt with: it isn't tried again.
        e.seen = Some(version);
        let (doc, page, index, turns, whole, path) = (e.doc, e.page, e.index, e.turns, e.whole_page, e.path.clone());
        let png = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| {
            let img = craft_image_codecs::decode(&bytes).map_err(|e| e.to_string())?;
            let (w, h) = img.dimensions();
            let (w, h, rgba) =
                pixels::turn_rgba(w, h, &img.to_rgba8(), (4 - turns) % 4).ok_or_else(|| tl!("The saved picture couldn't be read").to_string())?;
            png_of(w, h, &rgba)
        });
        let png = match png {
            Ok(p) => p,
            Err(e) => {
                self.notify_fmt("Couldn't read the edited picture: {e}", &[("e", &e)]);
                return;
            }
        };
        let Some(view) = self.views.iter().position(|v| v.id == doc) else { return };
        // The edit goes to the document the picture came from, shown so its change can be seen.
        self.active = Some(view);
        let name = path.file_name().map_or_else(|| "edited.png".to_owned(), |n| n.to_string_lossy().into_owned());
        let edit = match whole {
            // The whole page: a one-page PDF of the picture, at the size the page was shown, in
            // the page's place.
            Some(scale) => {
                let dpi = f64::from(72.0 * scale).clamp(1.0, 1200.0);
                match self.session.create_from_images_with_resolution(&[(name.clone(), png)], pdfcraft_engine::ImageResolution::Dpi(dpi)) {
                    Ok(bytes) => pdfcraft_engine::Edit::ReplacePages { pages: vec![page], name, bytes, src_pages: vec![0] },
                    Err(e) => {
                        self.notify_fmt("Couldn't read the edited picture: {e}", &[("e", &e.to_string())]);
                        return;
                    }
                }
            }
            None => pdfcraft_engine::Edit::EditPageImage {
                page,
                index,
                change: pdfcraft_engine::ImageEdit::Replace { name, bytes: std::sync::Arc::new(png) },
            },
        };
        if self.apply_edit(edit) {
            self.notify_fmt("Page {p} updated from the image editor", &[("p", &(page + 1).to_string())]);
        }
    }
}
