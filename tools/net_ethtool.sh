#!/bin/sh
ethtool "$@" 2>&1 | head -n 50
