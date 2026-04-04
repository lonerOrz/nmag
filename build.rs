use std::env;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("metadata.rs");

    let version = env!("CARGO_PKG_VERSION");
    let build_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| {
            // Simple UTC-like timestamp from epoch seconds
            let secs = d.as_secs() as i64;
            let (y, m, d, hh, mm, ss) = unix_epoch_to_utc(secs);
            format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
        })
        .unwrap_or_else(|_| "unknown".to_string());

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

/// Minimal UTC breakdown from Unix epoch seconds — no external crate needed.
fn unix_epoch_to_utc(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    // Algorithm from Howard Hinnant / civil_from_days
    let z = secs / 86_400 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    let rem = secs - (z - 719_468) * 86_400;
    let hh = (rem / 3600) as u32;
    let mm = ((rem % 3600) / 60) as u32;
    let ss = (rem % 60) as u32;
    (y, m as u32, d as u32, hh, mm, ss)
}
