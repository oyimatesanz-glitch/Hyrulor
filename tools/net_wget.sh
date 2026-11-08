#!/bin/sh
wget -qO- "$@" 2>&1 | head -n 50
