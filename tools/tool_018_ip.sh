#!/bin/sh
ip "$@" 2>&1 | head -n 50
