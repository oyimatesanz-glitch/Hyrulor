#!/bin/sh
rsync -avz "$@" 2>&1 | head -n 50
