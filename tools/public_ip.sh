#!/bin/sh
curl -s --max-time 3 ifconfig.me || echo "Sin conexión"
