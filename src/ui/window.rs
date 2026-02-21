use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{EventControllerKey, Label, ListView, ScrolledWindow, SearchEntry, SingleSelection};

use crate::config::Config;
use crate::db;
use crate::db::repository;
use crate::models::entry::{ClipboardEntry, ContentType};

use super::entry_object::EntryObject;
use super::entry_row;

pub fn truncate_preview(text: &str, max_chars: usize) -> (String, bool) {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        (text.to_string(), false)
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        (truncated, true)
    }
}

fn append_entries_to_store(
    store: &gio::ListStore,
    entries: &[ClipboardEntry],
    preview_chars: usize,
    image_max_px: i32,
) {
    for entry in entries {
        let id = entry.id.unwrap_or(0);
        let ct = entry.content_type.as_str();
        let created = entry.created_at.as_deref().unwrap_or("");

        let (preview, thumbnail) = if entry.content_type == ContentType::Image {
            let thumb = entry
                .blob_content
                .as_ref()
                .and_then(|blob| create_thumbnail_texture(blob, image_max_px));
            (String::new(), thumb)
        } else {
            let raw = entry.text_content.as_deref().unwrap_or("");
            let (mut text, was_truncated) = truncate_preview(raw, preview_chars);
            if was_truncated {
                text.push('…');
            }
            (text, None)
        };

        store.append(&EntryObject::new(id, &preview, ct, created, thumbnail));
    }
}

pub fn build_window(app: &gtk4::Application, config: &Config, db_path: PathBuf) {
    let page_size = config.history_page_size;
    let preview_chars = config.preview_text_chars;
    let image_max_px = config.image_preview_max_px;

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Clio History")
        .default_width(config.window_width)
        .default_height(config.window_height)
        .decorated(false)
        .build();

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // Search entry for filtering
    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Type to filter..."));
    main_box.append(&search_entry);

    // List store
    let store = gio::ListStore::new::<EntryObject>();

    // Load first page from DB
    let conn = match db::init_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error opening database: {e}");
            return;
        }
    };

    let entries = match repository::list_entries_page(&conn, page_size, 0) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error loading entries: {e}");
            return;
        }
    };

    let has_more = entries.len() >= page_size;
    append_entries_to_store(&store, &entries, preview_chars, image_max_px);

    // Pagination state
    let offset = Rc::new(RefCell::new(entries.len()));
    let all_loaded = Rc::new(RefCell::new(!has_more));
    // Current search query (empty = unfiltered)
    let search_query: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    // Placeholder for empty list
    let _placeholder = Label::new(Some("No clipboard history"));

    let selection = SingleSelection::new(Some(store.clone()));
    selection.set_autoselect(true);

    let factory = entry_row::create_factory();
    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);

    // DB-side search: on search_changed, clear store and query DB
    let store_for_search = store.clone();
    let db_path_for_search = db_path.clone();
    let offset_for_search = offset.clone();
    let all_loaded_for_search = all_loaded.clone();
    let search_query_for_search = search_query.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_string();
        *search_query_for_search.borrow_mut() = query.clone();
        store_for_search.remove_all();
        *offset_for_search.borrow_mut() = 0;
        *all_loaded_for_search.borrow_mut() = false;

        if let Ok(conn) = db::init_db(&db_path_for_search) {
            let entries = if query.is_empty() {
                repository::list_entries_page(&conn, page_size, 0).unwrap_or_default()
            } else {
                repository::search_entries_page(&conn, &query, page_size, 0).unwrap_or_default()
            };
            let has_more = entries.len() >= page_size;
            *offset_for_search.borrow_mut() = entries.len();
            *all_loaded_for_search.borrow_mut() = !has_more;
            append_entries_to_store(&store_for_search, &entries, preview_chars, image_max_px);
        }
    });

    search_entry.set_key_capture_widget(Some(&list_view));

    let scrolled = ScrolledWindow::new();
    scrolled.set_child(Some(&list_view));
    scrolled.set_vexpand(true);
    main_box.append(&scrolled);

    // Scroll-to-load: load next page when scrolled past 80%
    let store_for_scroll = store.clone();
    let db_path_for_scroll = db_path.clone();
    let offset_for_scroll = offset;
    let all_loaded_for_scroll = all_loaded;
    let search_query_for_scroll = search_query;
    let vadj = scrolled.vadjustment();
    vadj.connect_value_changed(move |adj| {
        if *all_loaded_for_scroll.borrow() {
            return;
        }
        let value = adj.value();
        let page = adj.page_size();
        let upper = adj.upper();
        if upper <= page {
            return;
        }
        let ratio = (value + page) / upper;
        if ratio < 0.8 {
            return;
        }

        let current_offset = *offset_for_scroll.borrow();
        if let Ok(conn) = db::init_db(&db_path_for_scroll) {
            let query = search_query_for_scroll.borrow().clone();
            let entries = if query.is_empty() {
                repository::list_entries_page(&conn, page_size, current_offset)
                    .unwrap_or_default()
            } else {
                repository::search_entries_page(&conn, &query, page_size, current_offset)
                    .unwrap_or_default()
            };
            let fetched = entries.len();
            if fetched < page_size {
                *all_loaded_for_scroll.borrow_mut() = true;
            }
            *offset_for_scroll.borrow_mut() = current_offset + fetched;
            append_entries_to_store(&store_for_scroll, &entries, preview_chars, image_max_px);
        }
    });

    // Handle Enter/click — select entry and set clipboard
    let conn_path = db_path.clone();
    let window_for_activate = window.clone();
    let sel_for_activate = selection.clone();
    list_view.connect_activate(move |_lv, position| {
        let item = sel_for_activate.item(position);
        if let Some(entry_obj) = item.and_then(|o| o.downcast::<EntryObject>().ok()) {
            let entry_id = entry_obj.id();
            if let Ok(conn) = db::init_db(&conn_path) {
                if let Ok(Some(entry)) = repository::get_entry_content(&conn, entry_id) {
                    match entry.content_type {
                        ContentType::Text => {
                            if let Some(text) = &entry.text_content {
                                let _ = crate::clipboard::write_clipboard_text(text);
                            }
                        }
                        ContentType::Image => {
                            if let Some(blob) = &entry.blob_content {
                                if let Ok(img) = image::load_from_memory(blob) {
                                    let rgba = img.to_rgba8();
                                    let (w, h) = rgba.dimensions();
                                    let _ = crate::clipboard::write_clipboard_image(
                                        rgba.as_raw(),
                                        w,
                                        h,
                                    );
                                }
                            }
                        }
                        ContentType::Unknown => {}
                    }
                    let _ = repository::update_timestamp(&conn, entry_id);
                }
            }
            window_for_activate.close();
        }
    });

    // Handle Delete key — remove selected entry
    let store_for_delete = store;
    let db_path_for_delete = db_path;
    let sel_for_delete = selection;
    let delete_controller = EventControllerKey::new();
    delete_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Delete {
            let selected = sel_for_delete.selected();
            if let Some(obj) = sel_for_delete.selected_item() {
                if let Ok(entry_obj) = obj.downcast::<EntryObject>() {
                    let entry_id = entry_obj.id();
                    if let Ok(conn) = db::init_db(&db_path_for_delete) {
                        let _ = repository::delete_entry(&conn, entry_id);
                    }
                    // Remove from backing store
                    let n = store_for_delete.n_items();
                    for i in 0..n {
                        if let Some(item) = store_for_delete.item(i) {
                            if let Ok(eo) = item.downcast::<EntryObject>() {
                                if eo.id() == entry_id {
                                    store_for_delete.remove(i);
                                    break;
                                }
                            }
                        }
                    }
                    // Select next item if available
                    let new_n = sel_for_delete.n_items();
                    if new_n > 0 && selected < new_n {
                        sel_for_delete.set_selected(selected);
                    }
                }
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    list_view.add_controller(delete_controller);

    // Handle Escape — close window (capture phase so SearchEntry doesn't intercept)
    let window_for_escape = window.clone();
    let escape_controller = EventControllerKey::new();
    escape_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    escape_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk4::gdk::Key::Escape {
            window_for_escape.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(escape_controller);

    window.set_child(Some(&main_box));
    window.present();
    list_view.grab_focus();
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
        // Each Cyrillic char is 1 char but 2 bytes
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

    // compute_thumbnail_size tests

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
        // 800x400, max 320 → scale 0.4 → 320x160
        assert_eq!(compute_thumbnail_size(800, 400, 320), (320, 160));
    }

    #[test]
    fn test_thumbnail_portrait() {
        // 400x800, max 320 → scale 0.4 → 160x320
        assert_eq!(compute_thumbnail_size(400, 800, 320), (160, 320));
    }

    #[test]
    fn test_thumbnail_square_large() {
        // 640x640, max 320 → scale 0.5 → 320x320
        assert_eq!(compute_thumbnail_size(640, 640, 320), (320, 320));
    }
}

/// Compute scaled thumbnail dimensions preserving aspect ratio.
/// If both dimensions are within `max_px`, returns original size.
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
