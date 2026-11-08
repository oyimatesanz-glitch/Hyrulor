#!/bin/sh
cargo --verbose "$@" 2>&1 | head -n 50
