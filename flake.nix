{
  description = "PIE8 bare-metal Raspberry Pi firmware development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          rust = pkgs.rust-bin.stable."1.85.0".default.override {
            extensions = [ "clippy" "llvm-tools" "rust-src" "rustfmt" ];
            targets = [ "aarch64-unknown-none" ];
          };
          x11Runtime = with pkgs.xorg; [
            libX11
            libXcursor
            libXext
            libXfixes
            libXi
            libXrandr
            libXrender
            libxcb
          ];
        in {
          default = pkgs.mkShell {
            packages = (with pkgs; [
              rust
              llvmPackages.bintools
              cargo-auditable
              curl
              git
              gnumake
              picocom
              dosfstools
              mtools
              python3
              util-linux
            ]) ++ x11Runtime;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath x11Runtime;
            RUST_BACKTRACE = "1";
          };
        });
    };
}
