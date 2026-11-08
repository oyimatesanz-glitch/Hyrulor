#!/bin/sh
bind -P "$@" 2>&1 | head -n 50
