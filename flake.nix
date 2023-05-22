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

          get-refs = prev.writeScriptBin "get-refs" ''
            ${prev.curl}/bin/curl --fail --silent \
              'https://monitoring.nixos.org/prometheus/api/v1/query?query=channel_revision' \
              | ${prev.jq}/bin/jq -r '{ "allowed_branches": [(.data.result[] | select(.metric.current == "1") | .metric.channel)] | sort, "max_days": 30 }' \
              > src/policy.json
          '';
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

            # Helpers
            get-refs
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
