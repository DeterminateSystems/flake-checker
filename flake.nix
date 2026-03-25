{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";

    crane.url = "https://flakehub.com/f/ipetkov/crane/0.20.3";

    easy-template = {
      url = "https://flakehub.com/f/DeterminateSystems/easy-template/0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
      version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";

      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forSystems =
        s: f:
        inputs.nixpkgs.lib.genAttrs s (
          system:
          f rec {
            inherit system;
            pkgs = import inputs.nixpkgs { inherit system; };
          }
        );

      forAllSystems = forSystems supportedSystems;
    in
    {
      packages = forAllSystems (
        { pkgs, system }:
        let
          # pkgsStatic gives musl on Linux and statically-linked C libs on macOS
          buildPkgs = pkgs.pkgsStatic;

          craneLib = inputs.crane.mkLib buildPkgs;

          src = builtins.path {
            name = "flake-checker-src";
            path = self;
          };

          commonArgs = {
            inherit src;
            CARGO_BUILD_TARGET = buildPkgs.stdenv.hostPlatform.rust.rustcTarget;
            "CC_${buildPkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}" =
              "${buildPkgs.stdenv.cc.targetPrefix}cc";
            "CARGO_TARGET_${buildPkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_LINKER" =
              "${buildPkgs.stdenv.cc.targetPrefix}cc";
          }
          // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        rec {
          default = flake-checker;
          flake-checker = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = true;
            }
          );
        }
      );

      devShells = forAllSystems (
        { pkgs, system }:
        {
          default =
            let
              check-nix-fmt = pkgs.writeShellApplication {
                name = "check-nix-fmt";
                runtimeInputs = with pkgs; [
                  git
                  nixfmt-rfc-style
                ];
                text = ''
                  git ls-files '*.nix' | xargs nixfmt --check
                '';
              };
              check-rust-fmt = pkgs.writeShellApplication {
                name = "check-rust-fmt";
                runtimeInputs = with pkgs; [
                  cargo
                  rustfmt
                ];
                text = "cargo fmt --check";
              };
              get-ref-statuses = pkgs.writeShellApplication {
                name = "get-ref-statuses";
                runtimeInputs = with pkgs; [
                  cargo
                  rustc
                ];
                text = "cargo run --features ref-statuses -- --get-ref-statuses";
              };
              update-readme = pkgs.writeShellApplication {
                name = "update-readme";
                runtimeInputs = [
                  inputs.easy-template.packages.${system}.default
                  pkgs.jq
                ];
                text = ''
                  tmp=$(mktemp -d)
                  inputs="''${tmp}/template-inputs.json"

                  jq '{supported: .}' ./ref-statuses.json > "''${inputs}"
                  easy-template ./templates/README.md.handlebars "''${inputs}" > README.md

                  rm -rf "''${tmp}"
                '';
              };
            in
            pkgs.mkShell {
              packages = with pkgs; [
                bashInteractive

                # Rust
                rustc
                cargo
                clippy
                rustfmt
                cargo-bloat
                cargo-edit
                cargo-machete
                cargo-watch
                rust-analyzer

                # CI checks
                check-nix-fmt
                check-rust-fmt

                # Scripts
                get-ref-statuses
                update-readme

                self.formatter.${system}
              ];

              env = {
                # Required by rust-analyzer
                RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
              };
            };
        }
      );

      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt-rfc-style);
    };
}
