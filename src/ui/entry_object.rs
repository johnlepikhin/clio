use std::cell::RefCell;

use glib::Properties;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::EntryObject)]
    pub struct EntryObject {
        #[property(get, set)]
        id: RefCell<i64>,
        #[property(get, set)]
        preview_text: RefCell<String>,
        #[property(get, set)]
        content_type: RefCell<String>,
        #[property(get, set)]
        created_at: RefCell<String>,
        #[property(get, set)]
        thumbnail: RefCell<Option<gdk::Texture>>,
        #[property(get, set)]
        source_app: RefCell<String>,
        #[property(get, set)]
        source_title: RefCell<String>,
        #[property(get, set)]
        expires_at: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryObject {
        const NAME: &'static str = "ClioEntryObject";
        type Type = super::EntryObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntryObject {}
}

glib::wrapper! {
    pub struct EntryObject(ObjectSubclass<imp::EntryObject>);
}

impl EntryObject {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        preview_text: &str,
        content_type: &str,
        created_at: &str,
        thumbnail: Option<gdk::Texture>,
        source_app: &str,
        source_title: &str,
        expires_at: &str,
    ) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("preview-text", preview_text)
            .property("content-type", content_type)
            .property("created-at", created_at)
            .property("thumbnail", thumbnail)
            .property("source-app", source_app)
            .property("source-title", source_title)
            .property("expires-at", expires_at)
            .build()
    }
}
