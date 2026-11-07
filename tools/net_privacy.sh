#!/usr/bin/env bash
DNS=$(resolvectl status 2>/dev/null | grep "Current DNS Server" | awk '{print $NF}' || echo "unknown")
CONN=$(ss -tun state established 2>/dev/null | wc -l)
echo "{\"dns_server\": \"$DNS\", \"active_connections\": $CONN}"
