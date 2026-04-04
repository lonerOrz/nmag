{
  description = "Screen magnifier for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        vulkan-icd = pkgs.lib.makeLibraryPath [ pkgs.vulkan-loader pkgs.libglvnd ];
      in
      {
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
