{ stdenv
, pkgs
, lib
, crane
, rust
, rust-bin
, nix-gitignore
, supportedSystems
, darwinFrameworks
}:

let
  inherit (stdenv.hostPlatform) system;

  nightlyVersion = "2024-06-13";
  rustNightly = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
    extensions = [ "rust-src" "rust-analyzer-preview" ];
    targets = cargoTargets;
  };

  # For easy cross-compilation in devShells
  # We are just composing the pkgsCross.*.stdenv.cc together
  crossPlatforms =
    let
      makeCrossPlatform = crossSystem:
        let
          pkgsCross =
            if crossSystem == system then pkgs
            else
              import pkgs.path {
                inherit system crossSystem;
                overlays = [ ];
              };

          rustTargetSpec = rust.toRustTargetSpec pkgsCross.pkgsStatic.stdenv.hostPlatform;
          rustTargetSpecUnderscored = builtins.replaceStrings [ "-" ] [ "_" ] rustTargetSpec;

          cargoLinkerEnv = lib.strings.toUpper "CARGO_TARGET_${rustTargetSpecUnderscored}_LINKER";
          cargoCcEnv = "CC_${rustTargetSpecUnderscored}"; # for ring

          cc = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
        in
        {
          name = crossSystem;
          value = {
            inherit rustTargetSpec cc;
            pkgs = pkgsCross;
            env = {
              "${cargoLinkerEnv}" = cc;
              "${cargoCcEnv}" = cc;
            };
          };
        };
      systems = lib.filter (s: s == system || lib.hasInfix "linux" s) supportedSystems;
    in
    builtins.listToAttrs (map makeCrossPlatform systems);

  cargoTargets = lib.mapAttrsToList (_: p: p.rustTargetSpec) crossPlatforms;
  cargoCrossEnvs = lib.foldl (acc: p: acc // p.env) { } (builtins.attrValues crossPlatforms);

  buildFor = system:
    let
      crossPlatform = crossPlatforms.${system};
      inherit (crossPlatform) pkgs;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustNightly;
      crateName = craneLib.crateNameFromCargoToml {
        cargoToml = ./Cargo.toml;
      };

      src = nix-gitignore.gitignoreSource [ ] ./.;

      commonArgs = {
        inherit (crateName) pname version;
        inherit src;

        buildInputs = with pkgs; [ ]
          ++ lib.optionals pkgs.stdenv.isDarwin darwinFrameworks;

        nativeBuildInputs = with pkgs; [ ]
          # The Rust toolchain from rust-overlay has a dynamic libiconv in depsTargetTargetPropagated
          # Our static libiconv needs to take precedence
          ++ lib.optionals pkgs.stdenv.isDarwin [
          libiconv
        ];

        cargoExtraArgs = "--target ${crossPlatform.rustTargetSpec}";

        cargoVendorDir = craneLib.vendorMultipleCargoDeps {
          inherit (craneLib.findCargoFiles src) cargoConfigs;
          cargoLockList = [
            ./Cargo.lock
            "${rustNightly.passthru.availableComponents.rust-src}/lib/rustlib/src/rust/Cargo.lock"
          ];
        };
      } // crossPlatform.env;

      crate = craneLib.buildPackage (commonArgs // {
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      } // lib.optionalAttrs (!stdenv.isDarwin) {
        allowedRequisites = [ ];
      });
    in
    crate;
in
{
  inherit crossPlatforms cargoTargets cargoCrossEnvs rustNightly;

  flake-checker = buildFor system;
}
