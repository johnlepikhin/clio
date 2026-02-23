/// Source application info: WM_CLASS and window title.
///
/// Best-effort detection: reads from the active window on X11.
/// Falls back to clipboard owner if active window detection fails.
/// Returns default (None, None) on Wayland or if detection fails.
#[derive(Debug, Default, Clone)]
pub struct SourceInfo {
    pub class: Option<String>,
    pub title: Option<String>,
}

#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
pub fn detect_source_app() -> SourceInfo {
    active_window_info().unwrap_or_else(|| clipboard_owner_info().unwrap_or_default())
}

/// Primary strategy: read WM_CLASS and title from _NET_ACTIVE_WINDOW.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn active_window_info() -> Option<SourceInfo> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;

    let active_atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;

    let reply = conn
        .get_property(false, root, active_atom, AtomEnum::ANY, 0, 1)
        .ok()?
        .reply()
        .ok()?;

    let window = reply.value32()?.next()?;
    if window == 0 {
        return None;
    }

    // Read title from the original top-level window
    let title = read_window_title(&conn, window);

    // WM_CLASS may be on the window itself or a parent
    let class = wm_class_with_traversal(&conn, window);

    // Return info only if at least class was found
    if class.is_some() || title.is_some() {
        Some(SourceInfo { class, title })
    } else {
        None
    }
}

/// Read WM_CLASS from `window`, walking up the parent chain (up to 10 levels).
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn wm_class_with_traversal(
    conn: &impl x11rb::protocol::xproto::ConnectionExt,
    window: u32,
) -> Option<String> {
    let mut current = window;
    for _ in 0..10 {
        if let Some(class) = read_wm_class(conn, current) {
            return Some(class);
        }
        // Move to parent
        let tree = conn.query_tree(current).ok()?.reply().ok()?;
        let parent = tree.parent;
        if parent == tree.root || parent == 0 {
            break;
        }
        current = parent;
    }
    None
}

/// Read WM_CLASS from a single window. Returns None if absent or empty.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn read_wm_class(
    conn: &impl x11rb::protocol::xproto::ConnectionExt,
    window: u32,
) -> Option<String> {
    use x11rb::properties::WmClass;

    let wm_class = WmClass::get(conn, window).ok()?.reply().ok()??;
    let class = std::str::from_utf8(wm_class.class()).ok()?;
    if class.is_empty() {
        return None;
    }
    Some(class.to_string())
}

/// Read window title: try _NET_WM_NAME (UTF8_STRING) first, fall back to WM_NAME.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn read_window_title(
    conn: &impl x11rb::protocol::xproto::ConnectionExt,
    window: u32,
) -> Option<String> {
    // Try _NET_WM_NAME first (UTF-8)
    let net_wm_name_atom = conn
        .intern_atom(false, b"_NET_WM_NAME")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let utf8_string_atom = conn
        .intern_atom(false, b"UTF8_STRING")
        .ok()?
        .reply()
        .ok()?
        .atom;

    let reply = conn
        .get_property(false, window, net_wm_name_atom, utf8_string_atom, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    if reply.value_len > 0 {
        let title = std::str::from_utf8(&reply.value).ok()?;
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }

    // Fallback: WM_NAME (STRING encoding)
    use x11rb::protocol::xproto::AtomEnum;
    let wm_name_atom: u32 = AtomEnum::WM_NAME.into();
    let string_atom: u32 = AtomEnum::STRING.into();
    let reply = conn
        .get_property(false, window, wm_name_atom, string_atom, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    if reply.value_len > 0 {
        let title = std::str::from_utf8(&reply.value).ok()?;
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }

    None
}

/// Fallback: read WM_CLASS and title from the clipboard selection owner.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn clipboard_owner_info() -> Option<SourceInfo> {
    use x11rb::protocol::xproto::ConnectionExt;

    let (conn, _) = x11rb::connect(None).ok()?;
    let clipboard_atom = conn
        .intern_atom(false, b"CLIPBOARD")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let owner = conn
        .get_selection_owner(clipboard_atom)
        .ok()?
        .reply()
        .ok()?
        .owner;
    if owner == 0 {
        return None;
    }
    let class = read_wm_class(&conn, owner);
    let title = read_window_title(&conn, owner);
    if class.is_some() || title.is_some() {
        Some(SourceInfo { class, title })
    } else {
        None
    }
}

#[cfg(not(all(target_os = "linux", feature = "x11-source-app")))]
pub fn detect_source_app() -> SourceInfo {
    SourceInfo::default()
}
