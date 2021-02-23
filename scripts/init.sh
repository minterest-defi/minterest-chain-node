#!/usr/bin/env bash

set -e

echo "*** Initializing WASM build environment"

if [ -z $CI_PROJECT_NAME ] ; then
    rustup update nightly
fi

rustup target add wasm32-unknown-unknown --toolchain nightly
rustup default nightly
