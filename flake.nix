{
  description = "A chess suite written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.bacon
            pkgs.just
            pkgs.cargo-nextest
            pkgs.cargo-flamegraph
            pkgs.fastchess
            pkgs.cutechess
          ];

          RUST_BACKTRACE = "full";

          # Optional: Add aliases for common commands if desired
          shellHook = ''
            echo "Using flake"
          '';
        };

        packages.eschec = pkgs.rustPlatform.buildRustPackage {
          pname = "eschec";
          version = "0.1.0"; # Or extract from Cargo.toml if more dynamic
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildInputs = [
            rustToolchain
          ];
        };

        defaultPackage = self.packages.${system}.eschec;
      });
}
