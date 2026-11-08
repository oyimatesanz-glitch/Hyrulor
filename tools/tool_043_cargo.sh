#!/bin/sh
cargo -v "$@" 2>&1 | head -n 50
