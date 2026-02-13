{
  description = "PicoForge Dev Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devenv.url = "github:cachix/devenv";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devenv.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "x86_64-darwin"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        {
          packages = import ./default.nix { inherit pkgs; };
          devenv.modules = [ ./devenv.nix ];
          devenv.shells = {
            default = { };
          };
        };
    };

  nixConfig = {
    extra-substituters = [
      "https://librekeys.cachix.org"
    ];
    extra-trusted-public-keys = [
      "librekeys.cachix.org-1:q+NyQsZgHyIMhYCIxyfpGs5jMU0/WHK7JTYgVbN3Iuk="
    ];
  };
}
