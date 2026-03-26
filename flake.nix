{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";

    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "https://flakehub.com/f/ipetkov/crane/0";

    easy-template = {
      url = "https://flakehub.com/f/DeterminateSystems/easy-template/0";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      inherit (inputs.nixpkgs) lib;

      lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
      version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";

      meta = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;

      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );
    in
    {
      packages = forAllSystems (
        { pkgs, system }:
        {
          default = self.packages.${system}.flake-checker;
          inherit (pkgs) flake-checker;
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
                  nixfmt
                ];
                text = ''
                  git ls-files '*.nix' | xargs nixfmt --check
                '';
              };
              check-rust-fmt = pkgs.writeShellApplication {
                name = "check-rust-fmt";
                runtimeInputs = with pkgs; [
                  rustToolchain
                ];
                text = "cargo fmt --check";
              };
              get-ref-statuses = pkgs.writeShellApplication {
                name = "get-ref-statuses";
                runtimeInputs = with pkgs; [
                  rustToolchain
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
                rustToolchain
                cargo-bloat
                cargo-edit
                cargo-machete
                cargo-watch

                # CI checks
                check-nix-fmt
                check-rust-fmt

                # Scripts
                get-ref-statuses
                update-readme

                self.formatter.${system}
              ];

              # Required by rust-analyzer
              env.RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
        }
      );

      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt);

      overlays.default =
        final: prev:
        let
          meta = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;

          inherit (prev.stdenv.hostPlatform) system;

          staticTarget =
            {
              "aarch64-linux" = "aarch64-unknown-linux-musl";
              "x86_64-linux" = "x86_64-unknown-linux-musl";
            }
            .${system} or null;

          rustToolchain =
            with inputs.fenix.packages.${system};
            combine (
              with stable;
              [
                clippy
                rustc
                cargo
                rustfmt
                rust-src
                rust-analyzer
              ]
              ++ lib.optionals (staticTarget != null) [
                targets.${staticTarget}.stable.rust-std
              ]
            );

          craneLib = (inputs.crane.mkLib prev).overrideToolchain rustToolchain;
        in
        {
          flake-checker = craneLib.buildPackage {
            inherit (meta) name;
            inherit version;
            src = builtins.path {
              name = "flake-checker-src";
              path = self;
            };
            env = lib.optionalAttrs (staticTarget != null) {
              CARGO_BUILD_TARGET = staticTarget;
            };
          };

          inherit rustToolchain;
        };
    };
}
