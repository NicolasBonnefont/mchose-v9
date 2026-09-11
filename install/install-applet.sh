#!/usr/bin/env bash
# Instala o applet na conta do usuario. Nao precisa de root.
#
# O `Exec=` do desktop entry recebe caminho ABSOLUTO: o painel lanca o processo,
# e resolver por PATH deixaria na mao do ambiente qual binario roda.
set -euo pipefail

ID=com.github.NicolasBonnefont.mchose-v9.Applet
RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DESTINO="${HOME}/.local/bin/cosmic-applet-mchose"
APPS="${HOME}/.local/share/applications"
PAINEL="${HOME}/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_wings"

BIN_ORIGEM="${1:-${RAIZ}/target/release/cosmic-applet-mchose}"
[ -x "$BIN_ORIGEM" ] || { echo "binario nao encontrado: $BIN_ORIGEM" >&2; exit 1; }

mkdir -p "$(dirname "$BIN_DESTINO")" "$APPS"
install -m755 "$BIN_ORIGEM" "$BIN_DESTINO"
sed "s|__BINARIO__|${BIN_DESTINO}|" \
  "${RAIZ}/install/${ID}.desktop" > "${APPS}/${ID}.desktop"
echo "instalado: ${BIN_DESTINO}"
echo "instalado: ${APPS}/${ID}.desktop"

# Acrescenta o id a primeira lista de plugins_wings, se ainda nao estiver la.
if [ -f "$PAINEL" ] && ! grep -q "$ID" "$PAINEL"; then
  cp "$PAINEL" "${PAINEL}.bak"
  python3 - "$PAINEL" "$ID" <<'PY'
import re, sys
caminho, ident = sys.argv[1], sys.argv[2]
texto = open(caminho).read()
# insere como primeiro item da lista da esquerda
novo = re.sub(r'Some\(\(\[\n', f'Some(([\n    "{ident}",\n', texto, count=1)
open(caminho, 'w').write(novo)
PY
  echo "acrescentado ao painel (backup em ${PAINEL}.bak)"
  echo "reinicie o painel:  pkill -f cosmic-panel"
else
  echo "ja estava no painel, ou plugins_wings nao existe"
fi
