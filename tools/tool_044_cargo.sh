#!/bin/sh
cargo -l "$@" 2>&1 | head -n 50
