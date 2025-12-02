#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Set app_id for Wayland - must be done BEFORE GTK init
    // This is required for GNOME GlobalShortcuts portal to accept our requests
    #[cfg(target_os = "linux")]
    {
        // Set the program name which GTK uses as app_id on Wayland
        // Must match the .desktop file name (without extension)
        gtk::glib::set_prgname(Some("ink.whis.Whis"));
        gtk::glib::set_application_name("Whis");
    }

    let args: Vec<String> = std::env::args().collect();

    // Handle --toggle command: send toggle to running instance and exit
    if args.contains(&"--toggle".to_string()) || args.contains(&"-t".to_string()) {
        if let Err(e) = whis_desktop::shortcuts::send_toggle_command() {
            eprintln!("Failed to toggle: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Handle --help
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("whis-desktop - Voice to text desktop application");
        println!();
        println!("USAGE:");
        println!("    whis-desktop [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -t, --toggle    Toggle recording in running instance");
        println!("    -h, --help      Print this help message");
        println!();
        println!("GLOBAL SHORTCUT:");
        println!("    Ctrl+Shift+R    Toggle recording (X11/Portal only)");
        println!();
        println!("For Wayland without portal support, configure your compositor");
        println!("to run 'whis-desktop --toggle' on your preferred shortcut.");
        return;
    }

    // Start the GUI application
    whis_desktop::run();
}
