const COMMANDS: &[&str] = &[
    "show_bubble",
    "hide_bubble",
    "is_bubble_visible",
    "request_overlay_permission",
    "has_overlay_permission",
    "set_bubble_recording",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
