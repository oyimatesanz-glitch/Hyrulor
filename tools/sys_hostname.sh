#!/bin/sh
hostname "$@" 2>&1 | head -n 50
