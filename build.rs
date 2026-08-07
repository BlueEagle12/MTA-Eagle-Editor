use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use image::imageops::FilterType;
use std::{env, fs::File, io::Write, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("gl_bindings.rs");
    let mut file = File::create(dest).unwrap();
    Registry::new(Api::Gl, (4, 6), Profile::Compatibility, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();

    println!("cargo:rerun-if-changed=assets/logo.png");
    let logo = image::open("assets/logo.png")
        .expect("assets/logo.png should be readable")
        .to_rgba8();
    let mut icon_file = File::create(Path::new(&out_dir).join("app_icon.rs")).unwrap();
    for (name, size) in [("SMALL", 16u32), ("MEDIUM", 32u32), ("BIG", 64u32)] {
        let resized = image::imageops::resize(&logo, size, size, FilterType::Lanczos3);
        writeln!(
            icon_file,
            "pub const {name}: [u8; {}] = {:?};",
            (size * size * 4),
            resized.into_raw()
        )
        .unwrap();
    }
}
