#!/bin/sh
cargo "$@" 2>&1 | head -n 50
