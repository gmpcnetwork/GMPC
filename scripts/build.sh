#!/usr/bin/env bash

set -e

./init.sh

PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null && pwd )"

export CARGO_INCREMENTAL=0

bold=$(tput bold)
normal=$(tput sgr0)

# Save current directory.
pushd . >/dev/null

for SRC in component/wasm
do
  echo "${bold}Building webassembly src in $SRC...${normal}"
  cd "$PROJECT_ROOT/$SRC"
  cd - >> /dev/null
done

cargo build --release

# Restore initial directory.
popd >/dev/null
