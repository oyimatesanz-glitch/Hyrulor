#!/bin/sh
md5sum "$@" 2>&1 | head -n 50
