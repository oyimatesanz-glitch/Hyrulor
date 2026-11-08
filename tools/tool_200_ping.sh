#!/bin/sh
ping -v "$@" 2>&1 | head -n 50
