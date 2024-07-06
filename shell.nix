let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.05";
  pkgs = import nixpkgs { config = {}; overlays = []; };
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    cargo
  ] ++ pkgs.lib.optionals pkg.stdenv.isDarwin [
    darwin.apple_sdk.frameworks.SystemConfiguration
    libiconv
  ];
}
