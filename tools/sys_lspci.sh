#!/bin/sh
lspci "$@" 2>&1 | head -n 50
