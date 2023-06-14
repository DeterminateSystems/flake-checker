#!/bin/bash

(
  cd $(git rev-parse --show-toplevel)

  mkdir -p releases

  # macOS binary
  echo "Building macOS binary"
  nix build .#packages.x86_64-darwin.default
  cp result/bin/flake-checker releases/flake-checker-X64-macOS
  echo "macOS binary: DONE"

  # Linux binary
  echo "Building Linux binary"
  sudo nix build .#packages.x86_64-linux.default
  cp result/bin/flake-checker releases/flake-checker-X64-Linux
  echo "Linux binary: DONE"

  # Now copy/paste into GitHub Releases
)
