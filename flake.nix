{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, ... }@inputs:
    let
      overlays = [
        inputs.rust-overlay.overlays.default
        (final: prev: {
          rustToolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        })
      ];
      systems = [ "aarch64-linux" "aarch64-darwin" "x86_64-linux" "x86_64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f {
        pkgs = import nixpkgs { inherit system overlays; };
      });
    in

    {
      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            cargo-edit
            cargo-watch
            rust-analyzer
          ];
        };

        ci = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
          ];
        };
      });

      packages = forAllSystems ({ pkgs }:
        let
          meta = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
          rust = pkgs.makeRustPlatform {
            cargo = pkgs.rustToolchain;
            rustc = pkgs.rustToolchain;
          };
        in
        {
          default =
            rust.buildRustPackage {
              pname = meta.name;
              version = meta.version;
              src = ./.;
              cargoHash = "sha256-toXBfFKKa1Vk3aeafPVLwHN3M5IW9BZckRv/9CLsJZA=";
            };
        });
    };
}
