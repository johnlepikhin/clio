/// Best-effort source application detection.
/// Returns the WM_CLASS of the active window on X11.
/// Falls back to clipboard owner if active window detection fails.
/// Returns None on Wayland or if detection fails.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
pub fn detect_source_app() -> Option<String> {
    active_window_class().or_else(clipboard_owner_class)
}

/// Primary strategy: read WM_CLASS from _NET_ACTIVE_WINDOW.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn active_window_class() -> Option<String> {
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

    // Try WM_CLASS on the active window, then traverse parents
    wm_class_with_traversal(&conn, window)
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

/// Fallback: read WM_CLASS from the clipboard selection owner.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
fn clipboard_owner_class() -> Option<String> {
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
    read_wm_class(&conn, owner)
}

#[cfg(not(all(target_os = "linux", feature = "x11-source-app")))]
pub fn detect_source_app() -> Option<String> {
    None
}
