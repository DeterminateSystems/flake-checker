{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
    };

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, ... }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f rec {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
          ];
        };

        cranePkgs = pkgs.callPackage ./crane.nix {
          inherit crane supportedSystems;
        };
      });
    in
    {
      packages = forAllSystems ({ cranePkgs, ... }: rec {
        inherit (cranePkgs) flake-checker;
        default = flake-checker;
      });

      devShells = forAllSystems ({ pkgs, cranePkgs }: {
        default =
          let
            check-nixpkgs-fmt = pkgs.writeShellApplication {
              name = "check-nixpkgs-fmt";
              runtimeInputs = with pkgs; [ git nixpkgs-fmt ];
              text = ''
                git ls-files '*.nix' | xargs nixpkgs-fmt --check
              '';
            };
            check-rustfmt = pkgs.writeShellApplication {
              name = "check-rustfmt";
              runtimeInputs = [ cranePkgs.rustNightly ];
              text = "cargo fmt --check";
            };
          in
          pkgs.mkShell {
            packages = (with pkgs; [
              bashInteractive

              # Rust
              cranePkgs.rustNightly
              cargo-bloat
              cargo-edit
              cargo-udeps
              cargo-edit
              cargo-watch
              rust-analyzer

              # Nix
              nixpkgs-fmt

              # CI checks
              check-nixpkgs-fmt
              check-rustfmt
            ]) ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [ Security ]);
          };
      });
    };
}
