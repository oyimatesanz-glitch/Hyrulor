#!/bin/sh
ss -tlnp "$@" 2>&1 | head -n 50
