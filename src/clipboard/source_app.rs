/// Best-effort source application detection.
/// Returns the WM_CLASS of the clipboard owner on X11.
/// Returns None on Wayland or if detection fails.
pub fn detect_source_app() -> Option<String> {
    // Source app detection is best-effort and X11-only.
    // For now, return None — implementation will use x11rb if available.
    // This avoids adding x11rb as a hard dependency.
    None
}
