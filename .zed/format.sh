#!/usr/bin/env bash
cargo fmt | leptosfmt --stdin # rustywind --output-css-file "$(pwd)/style/main.scss" --stdin
