{ pkgs ? import <nixpkgs> {} }:

let
  systemBuildInputs =
    if pkgs.stdenv.isDarwin then [
      pkgs.iconv
      pkgs.openssl
      pkgs.darwin.apple_sdk.frameworks.Security
      pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
    ] else [
      pkgs.iconv
      pkgs.openssl
    ];
in

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.rustfmt
    pkgs.openocd-rp2040
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
