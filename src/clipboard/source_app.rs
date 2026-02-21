/// Best-effort source application detection.
/// Returns the WM_CLASS of the clipboard owner on X11.
/// Returns None on Wayland or if detection fails.
#[cfg(all(target_os = "linux", feature = "x11-source-app"))]
pub fn detect_source_app() -> Option<String> {
    use x11rb::properties::WmClass;
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
    let wm_class = WmClass::get(&conn, owner).ok()?.reply().ok()??;
    let class = std::str::from_utf8(wm_class.class()).ok()?;
    if class.is_empty() {
        return None;
    }
    Some(class.to_string())
}

#[cfg(not(all(target_os = "linux", feature = "x11-source-app")))]
pub fn detect_source_app() -> Option<String> {
    None
}
