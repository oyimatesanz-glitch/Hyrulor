#!/bin/sh
compgen "$@" 2>&1 | head -n 50
