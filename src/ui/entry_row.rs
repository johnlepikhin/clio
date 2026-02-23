use chrono::{NaiveDateTime, Utc};
use gtk4::glib;
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
        meta_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
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
        let meta_text = build_meta_text(&entry_obj);

        let mask = entry_obj.mask_text();
        if !mask.is_empty() {
            // Mask overrides both text and image display
            thumbnail.set_paintable(gtk4::gdk::Paintable::NONE);
            thumbnail.set_size_request(-1, -1);
            thumbnail.set_visible(false);
            preview_label.set_text(&format!("\u{1F512} {mask}"));
            preview_label.add_css_class("masked");
            preview_label.set_visible(true);
        } else if ct == "image" {
            preview_label.set_text("");
            preview_label.remove_css_class("masked");
            preview_label.set_visible(false);
        } else {
            let display_text = if ct == "text" {
                entry_obj.preview_text().to_string()
            } else {
                "[Unknown content]".to_string()
            };
            preview_label.set_text(&display_text);
            preview_label.remove_css_class("masked");
            preview_label.set_visible(true);
        }
        meta_label.set_text(&meta_text);
        meta_label.set_visible(true);

        // Live update: refresh "expires in" when timer emits notify::expires-at
        let label_clone = meta_label.clone();
        let handler_id = entry_obj.connect_notify_local(Some("expires-at"), move |obj, _| {
            let eo: &EntryObject = obj.downcast_ref().unwrap();
            label_clone.set_text(&build_meta_text(eo));
        });
        // SAFETY: standard gtk4-rs pattern for storing handler ID on a ListItem
        unsafe {
            list_item.set_data("expires-handler", handler_id);
        }
    });

    factory.connect_unbind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let entry_obj = list_item.item().and_downcast::<EntryObject>().unwrap();
        if let Some(handler_id) =
            unsafe { list_item.steal_data::<glib::SignalHandlerId>("expires-handler") }
        {
            entry_obj.disconnect(handler_id);
        }
    });

    factory
}

fn build_meta_text(entry_obj: &EntryObject) -> String {
    let mut meta_text = format_created_at(&entry_obj.created_at());

    let source_app = entry_obj.source_app();
    if !source_app.is_empty() {
        meta_text.push_str(" | ");
        meta_text.push_str(&source_app);
        let source_title = entry_obj.source_title();
        if !source_title.is_empty() {
            meta_text.push_str(": ");
            meta_text.push_str(&source_title);
        }
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

    meta_text
}

fn format_created_at(raw: &str) -> String {
    let Ok(created) = NaiveDateTime::parse_from_str(raw, TIMESTAMP_FORMAT) else {
        return raw.to_string();
    };
    let now = Utc::now().naive_utc();
    let diff = now.signed_duration_since(created);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_hours() < 1 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{}d ago", diff.num_days())
    } else {
        created.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::models::entry::TIMESTAMP_FORMAT;

    fn ts_ago(secs: i64) -> String {
        (Utc::now().naive_utc() - chrono::Duration::seconds(secs))
            .format(TIMESTAMP_FORMAT)
            .to_string()
    }

    #[test]
    fn test_just_now() {
        assert_eq!(format_created_at(&ts_ago(5)), "just now");
        assert_eq!(format_created_at(&ts_ago(59)), "just now");
    }

    #[test]
    fn test_minutes_ago() {
        assert_eq!(format_created_at(&ts_ago(60)), "1m ago");
        assert_eq!(format_created_at(&ts_ago(300)), "5m ago");
        assert_eq!(format_created_at(&ts_ago(3599)), "59m ago");
    }

    #[test]
    fn test_hours_ago() {
        assert_eq!(format_created_at(&ts_ago(3600)), "1h ago");
        assert_eq!(format_created_at(&ts_ago(7200)), "2h ago");
    }

    #[test]
    fn test_days_ago() {
        assert_eq!(format_created_at(&ts_ago(86400)), "1d ago");
        assert_eq!(format_created_at(&ts_ago(86400 * 6)), "6d ago");
    }

    #[test]
    fn test_old_shows_date() {
        let result = format_created_at(&ts_ago(86400 * 30));
        assert!(result.starts_with("20"), "expected date, got: {result}");
        assert_eq!(result.len(), 10); // "YYYY-MM-DD"
    }

    #[test]
    fn test_invalid_input_passthrough() {
        assert_eq!(format_created_at("garbage"), "garbage");
    }
}
