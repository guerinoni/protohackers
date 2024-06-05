{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    cargo
    darwin.apple_sdk.frameworks.SystemConfiguration
    libiconv
  ];
}
