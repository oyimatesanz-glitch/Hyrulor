#!/bin/sh
which "$@" 2>&1 | head -n 50
