#!/bin/sh
ln -s "$@" 2>&1 | head -n 50
