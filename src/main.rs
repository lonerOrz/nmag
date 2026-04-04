mod render;
mod state;

use clap::Parser;

mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
}

#[derive(Parser)]
#[command(
    version = metadata::VERSION,
    long_version = metadata::LONG_VERSION,
    about = "Screen magnifier for Wayland",
)]
struct Cli {
    #[arg(short = 'z', long, default_value_t = 2.0)]
    zoom: f32,
    #[arg(short = 'r', long, default_value_t = 150.0)]
    radius: f32,
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let (mut st, mut eq) = state::State::setup(cli.zoom, cli.radius);
    loop {
        eq.blocking_dispatch(&mut st).unwrap();
        if st.quit {
            break;
        }
    }
}
