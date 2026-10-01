{
  description = "notifd — notification daemon backend for Quickshell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system: {
        default = (pkgsFor system).callPackage ./default.nix { };
      });

      devShells = forAllSystems (system: {
        default = (pkgsFor system).mkShell {
          packages = with pkgsFor system; [
            cargo
            rustc
            pkg-config
            rust-analyzer
            nixd
            nixfmt
            libcanberra-gtk3
            pulseaudio
          ];

          shellHook = ''
            export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-$HOME/.cache/notifd-target}"
            echo "notifd dev shell — build with: cargo build --release"
          '';
        };
      });

      formatter = forAllSystems (system: (pkgsFor system).nixfmt);
    };
}
