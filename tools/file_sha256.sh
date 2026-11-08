#!/bin/sh
sha256sum "$@" 2>&1 | head -n 50
