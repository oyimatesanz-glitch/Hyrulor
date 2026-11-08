#!/bin/sh
ps aux --sort=-%cpu 2>&1 | head -n 11
