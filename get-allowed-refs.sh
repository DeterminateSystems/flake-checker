#!/bin/bash

nix develop --command cargo run --features allowed-refs -- --get-allowed-refs
