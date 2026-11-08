#!/bin/sh
caller "$@" 2>&1 | head -n 50
