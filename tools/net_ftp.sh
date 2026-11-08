#!/bin/sh
ftp -inv "$@" 2>&1 | head -n 50
