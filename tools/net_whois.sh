#!/bin/sh
whois "$@" 2>&1 | head -n 50
