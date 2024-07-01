{
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2405.*";

    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1.*";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "https://flakehub.com/f/ipetkov/crane/0.14.*";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "flake-compat";
    };

    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/*";
    flake-schemas.url = "https://flakehub.com/f/DeterminateSystems/flake-schemas/*";
  };

  outputs = { self, nixpkgs, fenix, crane, flake-schemas, ... }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f rec {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ self.overlays.default ];
        };
      });
      meta = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
    in
    {
      inherit (flake-schemas) schemas;

      overlays.default = final: prev: rec {
        system = final.stdenv.hostPlatform.system;
        rustToolchain = with fenix.packages.${system};
          combine ([
            stable.clippy
            stable.rustc
            stable.cargo
            stable.rustfmt
            stable.rust-src
            stable.rust-analyzer-preview
          ] ++ nixpkgs.lib.optionals (system == "x86_64-linux") [
            targets.x86_64-unknown-linux-musl.stable.rust-std
          ] ++ nixpkgs.lib.optionals (system == "aarch64-linux") [
            targets.aarch64-unknown-linux-musl.stable.rust-std
          ]);
        craneLib = (crane.mkLib final).overrideToolchain rustToolchain;
        darwinInputs = final.lib.optionals final.stdenv.isDarwin (with final.darwin.apple_sdk.frameworks; [ Security SystemConfiguration ]);
      };

      packages = forAllSystems ({ pkgs }: rec {
        default = flake-checker;

        flake-checker =
          let
            args = {
              pname = meta.name;
              inherit (meta) version;
              src = self;
              doCheck = true;
              buildInputs = with pkgs; [ iconv ] ++ pkgs.darwinInputs;
            };
          in
          pkgs.craneLib.buildPackage (args // {
            cargoArtifacts = pkgs.craneLib.buildDepsOnly args;
          });
      });

      devShells = forAllSystems ({ pkgs }: {
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
              runtimeInputs = with pkgs; [ rustToolchain ];
              text = "cargo fmt --check";
            };
            get-allowed-refs = pkgs.writeShellApplication {
              name = "get-allowed-refs";
              runtimeInputs = with pkgs; [ rustToolchain ];
              text = "cargo run --features allowed-refs -- --get-allowed-refs";
            };
          in
          pkgs.mkShell {
            packages = (with pkgs; [
              bashInteractive

              # Rust
              rustToolchain
              cargo-bloat
              cargo-edit
              cargo-machete
              bacon
              rust-analyzer
              iconv

              # Nix
              nixpkgs-fmt

              # CI checks
              check-nixpkgs-fmt
              check-rustfmt

              # Scripts
              get-allowed-refs
            ]) ++ pkgs.darwinInputs;

            env = {
              # Required by rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
      });
    };
}
