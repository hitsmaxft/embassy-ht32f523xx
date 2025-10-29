{ pkgs ? import <nixpkgs> {} }:

let
  # Custom openocd package for HT32F523xx
  openocd-ht32f523xx = pkgs.callPackage (pkgs.fetchFromGitHub {
    owner = "hitsmaxft";
    repo = "openocd-ht32f523xx";
    rev = "main";  # You may want to pin to a specific commit
    hash = "sha256-lXQrow5p+sFCjDL9eLKRkdtvx4+1+FByPtkOyNloCYY=";  # This will fail first, then you can replace with the actual hash
  }) { pkgs=pkgs;};

  systemBuildInputs =
    if pkgs.stdenv.isDarwin then [
      pkgs.iconv
      pkgs.openssl
    ] else [
      pkgs.iconv
      pkgs.openssl
    ];
in

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.rustfmt
    #openocd-ht32f523xx
    pkgs.probe-rs-tools
    pkgs.cargo-make
    # pkgs.pre-commit
    # pkgs.rustPackages.clippy
  ] ++ systemBuildInputs;

  shellHook = ''
    export PATH=$PATH:$HOME/.cargo/bin
  '';

  RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
}
