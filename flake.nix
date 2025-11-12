{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/DeterminateSystems/secure/0";

    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "https://flakehub.com/f/nix-community/naersk/0";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );

      forAllSystems = forSystems supportedSystems;
    in
    {
      overlays.default =
        final: prev:
        let
          inherit (final.stdenv.hostPlatform) system;

          rustToolchain =
            with inputs.fenix.packages.${system};
            combine (
              [
                stable.clippy
                stable.rustc
                stable.cargo
                stable.rustfmt
                stable.rust-src
              ]
              ++ inputs.nixpkgs.lib.optionals (system == "x86_64-linux") [
                targets.x86_64-unknown-linux-musl.stable.rust-std
              ]
              ++ inputs.nixpkgs.lib.optionals (system == "aarch64-linux") [
                targets.aarch64-unknown-linux-musl.stable.rust-std
              ]
            );
        in
        {
          inherit rustToolchain;

          naerskLib = final.callPackage inputs.naersk {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
        };

      packages = forAllSystems (
        { pkgs, system }:
        rec {
          default = flake-checker;

          flake-checker = pkgs.naerskLib.buildPackage (
            {
              name = "flake-checker";
              src = self;
              doCheck = true;
              nativeBuildInputs = with pkgs; [ ] ++ lib.optionals stdenv.isDarwin [ libiconv ];
            }
            // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
              CARGO_BUILD_TARGET =
                if system == "x86_64-linux" then
                  "x86_64-unknown-linux-musl"
                else if system == "aarch64-linux" then
                  "aarch64-unknown-linux-musl"
                else
                  throw "Unsupported Linux system: ${system}";
              CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
            }
          );
        }
      );

      devShells = forAllSystems (
        { pkgs, system }:
        {
          default =
            let
              check-nixpkgs-fmt = pkgs.writeShellApplication {
                name = "check-nixpkgs-fmt";
                runtimeInputs = with pkgs; [
                  git
                  nixpkgs-fmt
                ];
                text = ''
                  nixpkgs-fmt --check "$(git ls-files '*.nix')"
                '';
              };
              check-rustfmt = pkgs.writeShellApplication {
                name = "check-rustfmt";
                runtimeInputs = with pkgs; [ rustToolchain ];
                text = "cargo fmt --check";
              };
              get-ref-statuses = pkgs.writeShellApplication {
                name = "get-ref-statuses";
                runtimeInputs = with pkgs; [ rustToolchain ];
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
                rust-analyzer

                # Nix
                nixpkgs-fmt

                # CI checks
                check-nixpkgs-fmt
                check-rustfmt

                # Scripts
                get-ref-statuses
                update-readme
              ];

              env = {
                # Required by rust-analyzer
                RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
              };
            };
        }
      );
    };
}
