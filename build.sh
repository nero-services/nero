#!/bin/bash
set -eu

cargo build

for d in plugins/*; do
    cd "$d" && cargo build
done
