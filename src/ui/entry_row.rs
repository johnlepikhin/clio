use gtk4::prelude::*;
use gtk4::{Align, Label, ListItem, Orientation, SignalListItemFactory};

use super::entry_object::EntryObject;

pub fn create_factory() -> SignalListItemFactory {
    let factory = SignalListItemFactory::new();

    factory.connect_setup(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();

        let hbox = gtk4::Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let thumbnail = gtk4::Image::new();
        thumbnail.set_pixel_size(48);
        thumbnail.set_widget_name("thumbnail");
        hbox.append(&thumbnail);

        let vbox = gtk4::Box::new(Orientation::Vertical, 2);
        vbox.set_hexpand(true);

        let preview_label = Label::new(None);
        preview_label.set_halign(Align::Start);
        preview_label.set_wrap(true);
        preview_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        preview_label.set_widget_name("preview");
        vbox.append(&preview_label);

        let meta_label = Label::new(None);
        meta_label.set_halign(Align::Start);
        meta_label.add_css_class("dim-label");
        meta_label.set_widget_name("meta");
        vbox.append(&meta_label);

        hbox.append(&vbox);
        list_item.set_child(Some(&hbox));
    });

    factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let entry_obj = list_item.item().and_downcast::<EntryObject>().unwrap();
        let hbox = list_item.child().and_downcast::<gtk4::Box>().unwrap();

        // Thumbnail
        let thumbnail = hbox.first_child().and_downcast::<gtk4::Image>().unwrap();
        if let Some(tex) = entry_obj.thumbnail() {
            thumbnail.set_paintable(Some(&tex));
            thumbnail.set_visible(true);
        } else {
            thumbnail.set_visible(false);
        }

        // Labels
        let vbox = thumbnail
            .next_sibling()
            .and_downcast::<gtk4::Box>()
            .unwrap();
        let preview_label = vbox.first_child().and_downcast::<Label>().unwrap();
        let meta_label = preview_label
            .next_sibling()
            .and_downcast::<Label>()
            .unwrap();

        let ct = entry_obj.content_type();
        let display_text = if ct == "text" {
            entry_obj.preview_text().to_string()
        } else if ct == "image" {
            "[Image]".to_string()
        } else {
            "[Unknown content]".to_string()
        };

        preview_label.set_text(&display_text);
        meta_label.set_text(&format!("{} | {}", ct, entry_obj.created_at()));
    });

    factory
}
