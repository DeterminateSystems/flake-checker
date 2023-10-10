{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2305.*.tar.gz";


    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    crane = {
      url = "https://flakehub.com/f/ipetkov/crane/0.14.*.tar.gz";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
    };

    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.0.1.tar.gz";
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
          darwinFrameworks = with pkgs.darwin.apple_sdk.frameworks; [ Security SystemConfiguration ];
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
            get-allowed-refs = pkgs.writeShellApplication {
              name = "get-allowed-refs";
              runtimeInputs = [ cranePkgs.rustNightly ];
              text = "cargo run --features allowed-refs -- --get-allowed-refs";
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

              # Scripts
              get-allowed-refs
            ]) ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [ Security ]);
          };
      });
    };
}
