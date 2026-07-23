{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          packages = [
            pkg-config
            dbus # For nostr-keyring

            just
            taplo
            cargo-llvm-cov
            python3Packages.diff-cover
          ];

          nativeBuildInputs = [
            (rust-bin.nightly.latest.default.override {
              extensions = [
                "llvm-tools-preview" # For coverage recipe
                "rust-src"
              ];
              targets = [ "wasm32-unknown-unknown" ];
            })
            rust-analyzer
          ];
        };
      }
    );
}
