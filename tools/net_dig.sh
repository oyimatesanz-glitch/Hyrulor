#!/bin/sh
dig "$@" 2>&1 | head -n 50
