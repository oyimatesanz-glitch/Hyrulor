#!/bin/sh
paste "$@" 2>&1 | head -n 50
