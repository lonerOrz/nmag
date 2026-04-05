{
  description = "Full-screen zoom for Wayland compositors";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        nmag = pkgs.rustPlatform.buildRustPackage {
          pname = "nmag";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];
          buildInputs = with pkgs; [
            wayland
            wayland-protocols
            libGL
            vulkan-loader
          ];
          postInstall = ''
            wrapProgram $out/bin/nmag \
              --prefix LD_LIBRARY_PATH : ${
                pkgs.lib.makeLibraryPath [
                  pkgs.libGL
                  pkgs.vulkan-loader
                ]
              }
          '';
          meta = {
            description = "Full-screen zoom for Wayland compositors";
            homepage = "https://github.com/lonerOrz/nmag";
            license = pkgs.lib.licenses.mit;
            platforms = pkgs.lib.platforms.linux;
            maintainers = with pkgs.lib.maintainers; [ lonerOrz ];
            mainProgram = "nmag";
          };
        };
        vulkan-icd = pkgs.lib.makeLibraryPath [
          pkgs.vulkan-loader
          pkgs.libglvnd
        ];
      in
      {
        packages.nmag = nmag;
        packages.default = self.packages.${system}.nmag;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            pkg-config
            wayland
            libxkbcommon
            vulkan-tools
            vulkan-validation-layers
          ];

          shellHook = ''
            # Add vulkan ICD and loader to library path
            export LD_LIBRARY_PATH="${vulkan-icd}:$LD_LIBRARY_PATH"

            # Use Lavapipe (software Vulkan) if no hardware driver
            if [ -f "${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64-linux.json" ]; then
              export VK_ICD_FILENAMES="${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.x86_64-linux.json"
            fi
          '';
        };
      }
    );
}
