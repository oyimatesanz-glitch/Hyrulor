#!/bin/sh
ifconfig "$@" 2>&1 | head -n 50
