#!/bin/sh
diff "$@" 2>&1 | head -n 50
