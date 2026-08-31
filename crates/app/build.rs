fn main() {
    // Иконка exe-файла в Windows (в taskbar/проводнике)
    #[cfg(target_env = "msvc")]
    {
        if let Err(e) = winresource::WindowsResource::new()
            .set_icon("../../assets/icon.ico")
            .set(
                "FileDescription",
                "SpeakerLab — loudspeaker enclosure design",
            )
            .compile()
        {
            println!("cargo:warning=winresource: {e}");
        }
    }
    #[cfg(not(target_env = "msvc"))]
    {
        let _ = winresource_stub();
    }
}

#[cfg(not(target_env = "msvc"))]
fn winresource_stub() {}
