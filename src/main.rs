mod config;
mod error;
mod render;
mod state;
mod types;

use clap::Parser;

mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
}

#[derive(Parser)]
#[command(
    version = metadata::VERSION,
    long_version = metadata::LONG_VERSION,
    about = "Full-screen magnifier for Wayland compositors",
)]
struct Cli {
    #[arg(short = 'z', long, default_value_t = config::DEFAULT_ZOOM)]
    zoom: f32,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let (mut st, mut eq) = state::State::setup(cli.zoom);
    loop {
        if let Err(e) = eq.blocking_dispatch(&mut st) {
            eprintln!("Wayland dispatch error: {e}");
            break;
        }
        if st.quit {
            break;
        }
    }
}
