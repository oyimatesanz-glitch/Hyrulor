#!/bin/sh
ps aux --sort=-%cpu | head -n 10
