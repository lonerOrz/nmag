{
  description = "Full-screen zoom for Wayland compositors";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      imports = [
        inputs.treefmt-nix.flakeModule
      ];

      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        let
          lib = pkgs.lib;

          buildInputs = with pkgs; [
            wayland
            wayland-protocols
            libGL
            vulkan-loader
            libxkbcommon
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];
        in
        {
          packages = {
            default = self'.packages.nmag;
            nmag = pkgs.rustPlatform.buildRustPackage {
              pname = "nmag";
              version = "0.1.0";
              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };
              inherit buildInputs nativeBuildInputs;
              postInstall = ''
                wrapProgram $out/bin/nmag \
                  --prefix LD_LIBRARY_PATH : ${
                    pkgs.lib.makeLibraryPath [
                      pkgs.libGL
                      pkgs.vulkan-loader
                    ]
                  }
              '';
              meta = with lib; {
                description = "Full-screen zoom for Wayland compositors";
                homepage = "https://github.com/lonerOrz/nmag";
                license = licenses.mit;
                mainProgram = "nmag";
                maintainers = with lib.maintainers; [ lonerOrz ];
                platforms = [
                  "x86_64-linux"
                  "aarch64-linux"
                ];
              };
            };
          };

          devShells.default = pkgs.mkShell {
            inherit buildInputs nativeBuildInputs;
            packages = with pkgs; [
              rustc
              cargo
              rust-analyzer
              rustfmt
              clippy
              vulkan-tools
              vulkan-validation-layers
              mesa
            ];

            env = {
              LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
              VK_ICD_FILENAMES = lib.optionalString (
                pkgs.stdenv.isLinux && pkgs.mesa ? out
              ) "${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64-linux.json";
            };
          };

          treefmt = {
            projectRootFile = "flake.nix";
            programs.nixfmt = {
              enable = true;
              package = pkgs.nixfmt-rfc-style;
            };
            programs.rustfmt.enable = true;
          };
        };
    };
}
