#!/bin/sh
env "$@" 2>&1 | head -n 50
