let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");

  pkgs = import <nixos-23.11> { overlays = [ rust_overlay ]; };
  unstable = import <nixos-unstable> { overlays = [ rust_overlay ]; };

  # rust = pkgs.rust-bin.stable."1.74.0".default.override {
  #   extensions = [ "rust-src" ];
  # };
  rust = unstable.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
    extensions = [ "rust-src" ];
  });

in
  pkgs.mkShell {
    nativeBuildInputs = [
    unstable.nixd # language server for nix files

    rust # overlay package (unstable)
    unstable.rust-analyzer

    ### dep ###
    # openssl
    # pkg-config
  ];

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
