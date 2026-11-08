#!/bin/sh
locate "$@" 2>&1 | head -n 50
