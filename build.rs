#[allow(dead_code)]
#[path = "src/icons.rs"]
mod icons;

fn main() {
    println!("cargo:rerun-if-changed=src/icons.rs");
    println!("cargo:rerun-if-changed=resources/logibar.rc");
    println!("cargo:rerun-if-changed=resources/logibar.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let frames = [16, 20, 24, 32, 40, 48, 64, 128, 256].map(|size| (size, icons::app_rgba(size)));
    std::fs::write(out.join("app.ico"), icons::ico(&frames)).unwrap();
    let manifest = std::env::current_dir()
        .unwrap()
        .join("resources/logibar.manifest");
    let resource = std::fs::read_to_string("resources/logibar.rc")
        .unwrap()
        .replace(
            "@ICON@",
            &out.join("app.ico").to_string_lossy().replace('\\', "/"),
        )
        .replace("@MANIFEST@", &manifest.to_string_lossy().replace('\\', "/"));
    std::fs::write(out.join("logibar.rc"), resource).unwrap();
    embed_resource::compile(out.join("logibar.rc"), embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
