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
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rust
              llvmPackages.bintools
              cargo-auditable
              curl
              git
              gnumake
              picocom
            ];
            RUST_BACKTRACE = "1";
          };
        });
    };
}
