#!/bin/sh
traceroute "$@" 2>&1 | head -n 50
