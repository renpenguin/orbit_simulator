{
  description = "eframe devShell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust = pkgs.rust-bin.stable."latest".default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [
            "rust-src" # for rust-analyzer
            "rustfmt"
            "clippy"
            "rust-analyzer"
          ];
        };
      in with pkgs; {
        devShells.default = mkShell rec {
          buildInputs = [
            # Rust
            rust
            trunk

            # misc. libraries
            openssl
            pkg-config

            # GUI libs
            libxkbcommon
            libGL
            fontconfig

            # wayland libraries
            wayland

            # x11 libraries
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11

          ];

          shellHook = ''
            export LD_LIBRARY_PATH=${lib.makeLibraryPath buildInputs}
          '';
        };
      }
  );
}
