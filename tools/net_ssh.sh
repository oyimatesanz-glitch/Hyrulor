#!/bin/sh
ssh -o BatchMode=yes "$@" 2>&1 | head -n 50
