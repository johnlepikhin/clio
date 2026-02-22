use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{EventControllerKey, ListView, ScrolledWindow, SearchEntry, SingleSelection};
use rusqlite::Connection;

use crate::config::Config;
use crate::db;
use crate::db::repository;
use crate::models::entry::{ClipboardEntry, ContentHash, EntryContent};

use super::entry_object::EntryObject;
use super::entry_row;

/// Content selected by the user for clipboard restore (passed out of GTK loop).
pub enum SelectedContent {
    Text(String),
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
}

/// Shared state for history window callbacks.
struct WindowState {
    conn: Rc<Connection>,
    store: gio::ListStore,
    offset: RefCell<usize>,
    all_loaded: RefCell<bool>,
    search_query: RefCell<String>,
    page_size: usize,
    preview_chars: usize,
    image_max_px: i32,
    /// Cache of decoded thumbnail textures keyed by content hash.
    thumbnail_cache: RefCell<HashMap<ContentHash, gtk4::gdk::Texture>>,
}

impl WindowState {
    /// Fetch a page of entries from DB (respects current search query).
    fn fetch_page(&self, offset: usize) -> Vec<ClipboardEntry> {
        let query = self.search_query.borrow().clone();
        if query.is_empty() {
            repository::list_entries_page(&self.conn, self.page_size, offset).unwrap_or_default()
        } else {
            repository::search_entries_page(&self.conn, &query, self.page_size, offset)
                .unwrap_or_default()
        }
    }

    /// Append entries to the backing ListStore.
    fn append_entries(&self, entries: &[ClipboardEntry]) {
        let mut cache = self.thumbnail_cache.borrow_mut();
        for entry in entries {
            let id = entry.id.unwrap_or(0);
            let ct = entry.content.content_type().as_str();
            let created = entry.created_at.as_deref().unwrap_or("");

            let (preview, thumbnail) = match &entry.content {
                EntryContent::Image(blob) => {
                    let thumb = if let Some(cached) = cache.get(&entry.content_hash) {
                        Some(cached.clone())
                    } else {
                        let decoded = create_thumbnail_texture(blob, self.image_max_px);
                        if let Some(ref texture) = decoded {
                            // Evict oldest entries when cache exceeds page_size
                            if cache.len() >= self.page_size {
                                if let Some(&key) = cache.keys().next() {
                                    cache.remove(&key);
                                }
                            }
                            cache.insert(entry.content_hash, texture.clone());
                        }
                        decoded
                    };
                    (String::new(), thumb)
                }
                EntryContent::Text(text) => {
                    let (mut preview, was_truncated) =
                        truncate_preview(text, self.preview_chars);
                    if was_truncated {
                        preview.push('…');
                    }
                    (preview, None)
                }
            };

            let source_app = entry.source_app.as_deref().unwrap_or("");
            let expires_at = entry.expires_at.as_deref().unwrap_or("");

            self.store.append(&EntryObject::new(
                id, &preview, ct, created, thumbnail, source_app, expires_at,
            ));
        }
    }

    /// Reset pagination, clear store, load first page.
    fn reload(&self) {
        self.store.remove_all();
        self.thumbnail_cache.borrow_mut().clear();
        *self.offset.borrow_mut() = 0;
        *self.all_loaded.borrow_mut() = false;

        let entries = self.fetch_page(0);
        let has_more = entries.len() >= self.page_size;
        *self.offset.borrow_mut() = entries.len();
        *self.all_loaded.borrow_mut() = !has_more;
        self.append_entries(&entries);
    }

    /// Load the next page (for infinite scroll).
    fn load_next_page(&self) {
        if *self.all_loaded.borrow() {
            return;
        }
        let current_offset = *self.offset.borrow();
        let entries = self.fetch_page(current_offset);
        let fetched = entries.len();
        if fetched < self.page_size {
            *self.all_loaded.borrow_mut() = true;
        }
        *self.offset.borrow_mut() = current_offset + fetched;
        self.append_entries(&entries);
    }
}

pub fn truncate_preview(text: &str, max_chars: usize) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        (text.to_string(), false)
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        (truncated, true)
    }
}

pub fn build_window(
    app: &gtk4::Application,
    config: &Config,
    db_path: PathBuf,
    selected: Rc<RefCell<Option<SelectedContent>>>,
) {
    let conn = match db::init_db_ui(&db_path) {
        Ok(c) => Rc::new(c),
        Err(e) => {
            eprintln!("error opening database: {e}");
            return;
        }
    };

    let store = gio::ListStore::new::<EntryObject>();

    let state = Rc::new(WindowState {
        conn,
        store: store.clone(),
        offset: RefCell::new(0),
        all_loaded: RefCell::new(false),
        search_query: RefCell::new(String::new()),
        page_size: config.history_page_size,
        preview_chars: config.preview_text_chars,
        image_max_px: config.image_preview_max_px,
        thumbnail_cache: RefCell::new(HashMap::new()),
    });

    // Load first page
    let entries = match repository::list_entries_page(&state.conn, state.page_size, 0) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error loading entries: {e}");
            return;
        }
    };
    let has_more = entries.len() >= state.page_size;
    *state.offset.borrow_mut() = entries.len();
    *state.all_loaded.borrow_mut() = !has_more;
    state.append_entries(&entries);

    // Build widgets
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Clio History")
        .default_width(config.window_width)
        .default_height(config.window_height)
        .decorated(false)
        .build();

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Type to filter..."));
    main_box.append(&search_entry);

    let selection = SingleSelection::new(Some(store.clone()));
    selection.set_autoselect(true);

    let factory = entry_row::create_factory();
    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);
    list_view.set_show_separators(true);

    setup_search(&search_entry, &state);
    search_entry.set_key_capture_widget(Some(&list_view));

    let scrolled = ScrolledWindow::new();
    scrolled.set_child(Some(&list_view));
    scrolled.set_vexpand(true);
    main_box.append(&scrolled);

    setup_scroll(&scrolled, &state);
    setup_activate(&list_view, &state, &selection, &window, selected);
    setup_delete(&list_view, &state, &selection);
    setup_escape(&window);

    window.set_child(Some(&main_box));
    window.present();
    list_view.grab_focus();
    if store.n_items() > 0 {
        list_view.scroll_to(0, gtk4::ListScrollFlags::FOCUS, None::<gtk4::ScrollInfo>);
    }
}

fn setup_search(search_entry: &SearchEntry, state: &Rc<WindowState>) {
    let state = state.clone();
    search_entry.connect_search_changed(move |entry| {
        *state.search_query.borrow_mut() = entry.text().to_string();
        state.reload();
    });
}

fn setup_scroll(scrolled: &ScrolledWindow, state: &Rc<WindowState>) {
    let state = state.clone();
    let vadj = scrolled.vadjustment();
    vadj.connect_value_changed(move |adj| {
        let value = adj.value();
        let page = adj.page_size();
        let upper = adj.upper();
        if upper <= page {
            return;
        }
        let ratio = (value + page) / upper;
        if ratio >= 0.8 {
            state.load_next_page();
        }
    });
}

fn setup_activate(
    list_view: &ListView,
    state: &Rc<WindowState>,
    selection: &SingleSelection,
    window: &gtk4::ApplicationWindow,
    selected: Rc<RefCell<Option<SelectedContent>>>,
) {
    let state = state.clone();
    let sel = selection.clone();
    let win = window.clone();
    list_view.connect_activate(move |_lv, position| {
        let item = sel.item(position);
        if let Some(entry_obj) = item.and_then(|o| o.downcast::<EntryObject>().ok()) {
            let entry_id = entry_obj.id();
            if let Ok(Some(entry)) = repository::get_entry_content(&state.conn, entry_id) {
                match &entry.content {
                    EntryContent::Text(text) => {
                        *selected.borrow_mut() = Some(SelectedContent::Text(text.clone()));
                    }
                    EntryContent::Image(blob) => {
                        if let Ok(img) = image::load_from_memory(blob) {
                            let rgba = img.to_rgba8();
                            let (w, h) = rgba.dimensions();
                            *selected.borrow_mut() = Some(SelectedContent::Image {
                                rgba: rgba.into_raw(),
                                width: w,
                                height: h,
                            });
                        }
                    }
                }
                let _ = repository::update_timestamp_and_expiry(&state.conn, entry_id, None);
            }
            win.close();
        }
    });
}

fn setup_delete(
    list_view: &ListView,
    state: &Rc<WindowState>,
    selection: &SingleSelection,
) {
    let state = state.clone();
    let sel = selection.clone();
    let controller = EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Delete {
            let selected = sel.selected();
            if let Some(obj) = sel.selected_item() {
                if let Ok(entry_obj) = obj.downcast::<EntryObject>() {
                    let _ = repository::delete_entry(&state.conn, entry_obj.id());
                    state.store.remove(selected);
                    let new_n = sel.n_items();
                    if new_n > 0 && selected < new_n {
                        sel.set_selected(selected);
                    }
                }
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    list_view.add_controller(controller);
}

fn setup_escape(window: &gtk4::ApplicationWindow) {
    let win = window.clone();
    let controller = EventControllerKey::new();
    controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            win.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(controller);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_text() {
        let (result, truncated) = truncate_preview("hello", 10);
        assert_eq!(result, "hello");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_exact_boundary() {
        let (result, truncated) = truncate_preview("abcde", 5);
        assert_eq!(result, "abcde");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_long_text() {
        let (result, truncated) = truncate_preview("abcdefghij", 5);
        assert_eq!(result, "abcde");
        assert!(truncated);
    }

    #[test]
    fn test_truncate_multibyte_utf8() {
        let (result, truncated) = truncate_preview("Привет мир!", 6);
        assert_eq!(result, "Привет");
        assert!(truncated);
    }

    #[test]
    fn test_truncate_emoji() {
        let (result, truncated) = truncate_preview("😀😁😂🤣😃", 3);
        assert_eq!(result, "😀😁😂");
        assert!(truncated);
    }

    #[test]
    fn test_truncate_empty() {
        let (result, truncated) = truncate_preview("", 10);
        assert_eq!(result, "");
        assert!(!truncated);
    }

    #[test]
    fn test_thumbnail_small_image_no_scaling() {
        assert_eq!(compute_thumbnail_size(100, 80, 320), (100, 80));
    }

    #[test]
    fn test_thumbnail_exact_boundary() {
        assert_eq!(compute_thumbnail_size(320, 320, 320), (320, 320));
    }

    #[test]
    fn test_thumbnail_landscape() {
        assert_eq!(compute_thumbnail_size(800, 400, 320), (320, 160));
    }

    #[test]
    fn test_thumbnail_portrait() {
        assert_eq!(compute_thumbnail_size(400, 800, 320), (160, 320));
    }

    #[test]
    fn test_thumbnail_square_large() {
        assert_eq!(compute_thumbnail_size(640, 640, 320), (320, 320));
    }
}

/// Compute scaled thumbnail dimensions preserving aspect ratio.
pub fn compute_thumbnail_size(src_w: i32, src_h: i32, max_px: i32) -> (i32, i32) {
    if src_w <= max_px && src_h <= max_px {
        return (src_w, src_h);
    }
    let max_side = src_w.max(src_h) as f64;
    let scale = max_px as f64 / max_side;
    let dst_w = (src_w as f64 * scale) as i32;
    let dst_h = (src_h as f64 * scale) as i32;
    (dst_w.max(1), dst_h.max(1))
}

fn create_thumbnail_texture(png_bytes: &[u8], max_px: i32) -> Option<gtk4::gdk::Texture> {
    use gtk4::gdk_pixbuf::InterpType;

    let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
    pixbuf_loader.write(png_bytes).ok()?;
    pixbuf_loader.close().ok()?;
    let pixbuf = pixbuf_loader.pixbuf()?;

    let src_w = pixbuf.width();
    let src_h = pixbuf.height();
    let (dst_w, dst_h) = compute_thumbnail_size(src_w, src_h, max_px);

    if dst_w == src_w && dst_h == src_h {
        return Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf));
    }

    let scaled = pixbuf.scale_simple(dst_w, dst_h, InterpType::Bilinear)?;
    Some(gtk4::gdk::Texture::for_pixbuf(&scaled))
}
