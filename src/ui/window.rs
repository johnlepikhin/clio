use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    CustomFilter, EventControllerKey, FilterChange, FilterListModel, Label, ListView,
    ScrolledWindow, SearchEntry, SingleSelection,
};

use crate::config::Config;
use crate::db;
use crate::db::repository;
use crate::models::entry::ContentType;

use super::entry_object::EntryObject;
use super::entry_row;

pub fn build_window(app: &gtk4::Application, config: &Config, db_path: PathBuf) {
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

    // Load entries from DB
    let conn = match db::init_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error opening database: {e}");
            return;
        }
    };

    let entries = match repository::list_entries(&conn, config.max_history) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error loading entries: {e}");
            return;
        }
    };

    for entry in &entries {
        let id = entry.id.unwrap_or(0);
        let preview = entry.text_content.as_deref().unwrap_or("");
        let ct = entry.content_type.as_str();
        let created = entry.created_at.as_deref().unwrap_or("");

        let thumbnail = if entry.content_type == ContentType::Image {
            entry
                .blob_content
                .as_ref()
                .and_then(|blob| create_thumbnail_texture(blob))
        } else {
            None
        };

        store.append(&EntryObject::new(id, preview, ct, created, thumbnail));
    }

    // Placeholder for empty list
    let _placeholder = Label::new(Some("No clipboard history"));

    // Filter
    let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let filter_text_clone = filter_text.clone();
    let filter = CustomFilter::new(move |obj| {
        let entry_obj = obj.downcast_ref::<EntryObject>().unwrap();
        let ft = filter_text_clone.borrow();
        if ft.is_empty() {
            return true;
        }
        if entry_obj.content_type() != "text" {
            return false;
        }
        entry_obj
            .preview_text()
            .to_lowercase()
            .contains(&ft.to_lowercase())
    });

    let filter_model = FilterListModel::new(Some(store.clone()), Some(filter.clone()));
    let selection = SingleSelection::new(Some(filter_model));
    selection.set_autoselect(true);

    let factory = entry_row::create_factory();
    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);

    // Connect search to filter
    let filter_text_for_search = filter_text;
    let filter_for_search = filter;
    search_entry.connect_search_changed(move |entry| {
        *filter_text_for_search.borrow_mut() = entry.text().to_string();
        filter_for_search.changed(FilterChange::Different);
    });

    search_entry.set_key_capture_widget(Some(&list_view));

    let scrolled = ScrolledWindow::new();
    scrolled.set_child(Some(&list_view));
    scrolled.set_vexpand(true);
    main_box.append(&scrolled);

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

    // Handle Escape — close window
    let window_for_escape = window.clone();
    let escape_controller = EventControllerKey::new();
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
}

fn create_thumbnail_texture(png_bytes: &[u8]) -> Option<gtk4::gdk::Texture> {
    let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
    pixbuf_loader.write(png_bytes).ok()?;
    pixbuf_loader.close().ok()?;
    let pixbuf = pixbuf_loader.pixbuf()?;
    Some(gtk4::gdk::Texture::for_pixbuf(&pixbuf))
}
