use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("metadata.rs");

    let version = env!("CARGO_PKG_VERSION");
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z");

    let content = format!(
        r#"pub const VERSION: &str = "{version}";
pub const LONG_VERSION: &str = concat!(
    "{version}",
    "\nbuild-time: {build_time}",
);
"#,
    );

    fs::write(&dest, content).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
