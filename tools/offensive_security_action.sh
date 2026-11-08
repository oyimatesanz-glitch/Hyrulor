#!/bin/sh
# Herramienta de seguridad ofensiva: DRY-RUN aislado.
#
# Esta herramienta NUNCA ejecuta un ataque real. Su único objetivo es
# demostrar el circuito de autorización humana + sandbox + auditoría.
# El daemon solo la invoca después de que la persona autoriza el formulario.
#
# Sandbox del daemon (bubblewrap):
#   --unshare-user --unshare-uts --unshare-ipc --unshare-pid --unshare-net
#   --cap-drop ALL --clearenv --die-with-parent
#
# Sin red, sin capabilities, sin acceso al HOME: si algún día se conecta
# una herramienta real, el aislamiento ya está garantizado por el sandbox.

set -eu

ARGS="${1:-{\}}"

printf 'DRY-RUN OK: offensive_security_action recibida y auditada.\n'
printf 'No se ejecutó ningún ataque real. Entorno: bubblewrap sin red y sin capabilities.\n'
printf 'Argumentos autorizados: %s\n' "$ARGS"
