fn main() {
    // Link AAudio library for Android
    #[cfg(target_os = "android")]
    {
        println!("cargo:rustc-link-lib=aaudio");
        println!("cargo:rustc-link-lib=c++_shared");
    }

    tauri_build::build()
}
