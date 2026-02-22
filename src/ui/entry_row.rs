use chrono::{NaiveDateTime, Utc};
use gtk4::prelude::*;
use gtk4::{Align, Label, ListItem, Orientation, SignalListItemFactory};

use crate::models::entry::TIMESTAMP_FORMAT;

use super::entry_object::EntryObject;

pub fn create_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();

        let root_vbox = gtk4::Box::new(Orientation::Vertical, 4);
        root_vbox.set_margin_top(4);
        root_vbox.set_margin_bottom(4);
        root_vbox.set_margin_start(8);
        root_vbox.set_margin_end(8);

        let thumbnail = gtk4::Image::new();
        thumbnail.set_widget_name("thumbnail");
        thumbnail.set_halign(Align::Start);
        root_vbox.append(&thumbnail);

        let labels_vbox = gtk4::Box::new(Orientation::Vertical, 2);
        labels_vbox.set_hexpand(true);

        let preview_label = Label::new(None);
        preview_label.set_halign(Align::Start);
        preview_label.set_wrap(true);
        preview_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        preview_label.set_widget_name("preview");
        labels_vbox.append(&preview_label);

        let meta_label = Label::new(None);
        meta_label.set_halign(Align::Start);
        meta_label.add_css_class("dim-label");
        meta_label.set_widget_name("meta");
        labels_vbox.append(&meta_label);

        root_vbox.append(&labels_vbox);
        list_item.set_child(Some(&root_vbox));
    });

    factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let entry_obj = list_item.item().and_downcast::<EntryObject>().unwrap();
        let root_vbox = list_item.child().and_downcast::<gtk4::Box>().unwrap();

        // Thumbnail (first child of root_vbox)
        let thumbnail = root_vbox
            .first_child()
            .and_downcast::<gtk4::Image>()
            .unwrap();
        if let Some(tex) = entry_obj.thumbnail() {
            thumbnail.set_paintable(Some(&tex));
            thumbnail.set_size_request(tex.width(), tex.height());
            thumbnail.set_visible(true);
        } else {
            thumbnail.set_paintable(gtk4::gdk::Paintable::NONE);
            thumbnail.set_size_request(-1, -1);
            thumbnail.set_visible(false);
        }

        // Labels (second child of root_vbox)
        let labels_vbox = thumbnail
            .next_sibling()
            .and_downcast::<gtk4::Box>()
            .unwrap();
        let preview_label = labels_vbox
            .first_child()
            .and_downcast::<Label>()
            .unwrap();
        let meta_label = preview_label
            .next_sibling()
            .and_downcast::<Label>()
            .unwrap();

        let ct = entry_obj.content_type();
        let mut meta_text = format!("{} | {}", ct, entry_obj.created_at());

        let source_app = entry_obj.source_app();
        if !source_app.is_empty() {
            meta_text.push_str(" | ");
            meta_text.push_str(&source_app);
        }

        let expires_at = entry_obj.expires_at();
        if !expires_at.is_empty() {
            if let Ok(expires) = NaiveDateTime::parse_from_str(&expires_at, TIMESTAMP_FORMAT) {
                let now = Utc::now().naive_utc();
                if expires > now {
                    let remaining = expires - now;
                    let std_dur =
                        std::time::Duration::from_secs(remaining.num_seconds().unsigned_abs());
                    let formatted = humantime::format_duration(std_dur).to_string();
                    meta_text.push_str(" | expires in ");
                    meta_text.push_str(&formatted);
                } else {
                    meta_text.push_str(" | expired");
                }
            }
        }

        if ct == "image" {
            // Image is the preview; hide preview_label, show only meta
            preview_label.set_text("");
            preview_label.set_visible(false);
            meta_label.set_text(&meta_text);
            meta_label.set_visible(true);
        } else {
            let display_text = if ct == "text" {
                entry_obj.preview_text().to_string()
            } else {
                "[Unknown content]".to_string()
            };
            preview_label.set_text(&display_text);
            preview_label.set_visible(true);
            meta_label.set_text(&meta_text);
            meta_label.set_visible(true);
        }
    });

    factory
}
