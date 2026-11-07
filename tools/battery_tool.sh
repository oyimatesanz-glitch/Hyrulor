#!/bin/sh
BAT=$(ls -d /sys/class/power_supply/BAT* 2>/dev/null | head -n1)

if [ -z "$BAT" ]; then
    echo '{"error": "No se detectó batería"}'
    exit 1
fi

cap=$(cat "$BAT/capacity" 2>/dev/null || echo "0")
stat=$(cat "$BAT/status" 2>/dev/null || echo "Desconocido")

case "$stat" in
    Discharging) stat="Descargando" ;;
    Charging) stat="Cargando" ;;
    Full) stat="Completa" ;;
    "Not charging") stat="Sin cargar" ;;
esac

echo "{\"level\": $cap, \"status\": \"$stat\", \"health\": \"ok\"}"
