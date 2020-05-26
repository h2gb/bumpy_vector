#!/bin/bash

set -euo pipefail

# Print errors in red
err() {
  echo -ne '\e[31m\e[1m' # Red + Bold
  echo -e "$@"
  echo -ne '\e[0m'
  exit 1
}

# Get the root directory
BASE=$(git rev-parse --show-toplevel)/src

echo "Running tests..."
pushd $BASE
cargo test --all-features || err "One or more tests failed!"
popd
