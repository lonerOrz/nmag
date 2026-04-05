# nmag

Full-screen zoom application for Wayland compositors.

## Features

- **Full-screen zoom** - Replaces the traditional circular magnifier with a full-screen view
- **Pan/drag support** - Click and drag to pan around the zoomed view
- **Smooth rendering** - GPU-accelerated rendering via wgpu
- **Configurable zoom level** - Set zoom via command-line argument

## Requirements

- Wayland compositor (Hyprland, GNOME, KDE, etc.)
- wlroots-based compositors recommended for best experience

### Build Dependencies

- Rust (stable)
- pkg-config
- libwayland-dev
- libxkbcommon-dev
- libgl-dev

## Installation

### From Source

```bash
cargo build --release
```

The binary will be at `target/release/nmag`.

### With Nix

```bash
nix run github:lonerOrz/nmag
```

Or add to your flake:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nmag.url = "github:lonerOrz/nmag";
  };

  outputs =
    inputs@{
      self,
      flake-utils,
      nixpkgs,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [ inputs.nmag.packages.${system}.nmag ];
        };
      }
    );
}
```

## Usage

```bash
# Run with default zoom (2.0x)
nmag

# Run with custom zoom level
nmag -z 3.0
```

### Controls

- **Click + Drag** - Pan the zoomed view
- **Scroll wheel** - Adjust zoom level (if supported by compositor)

---

## Development

Contributions and feedback are welcome!
Please format code with `cargo fmt` and check with `cargo clippy`.

---

## License

This project is licensed under the BSD 3-Clause License.

---

> If you find `nmag` useful, please give it a ⭐ and share! 🎉
