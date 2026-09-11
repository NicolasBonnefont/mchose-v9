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
# Aspas no Exec e escape do sed: HOME com espaco faria o painel lancar so a
# primeira palavra do caminho, e um `&` viraria o texto casado na substituicao.
ESCAPADO=$(printf '%s' "$BIN_DESTINO" | sed 's/[&|\\]/\\&/g')
sed "s|__BINARIO__|\"${ESCAPADO}\"|" \
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
  # `re.sub` que nao casa reescreve o arquivo identico e sairia em silencio:
  # confere o resultado em vez de confiar no comando.
  if grep -q "$ID" "$PAINEL"; then
    echo "acrescentado ao painel (backup em ${PAINEL}.bak)"
  else
    echo "NAO consegui editar ${PAINEL} — formato inesperado." >&2
    echo "Acrescente \"${ID}\" a mao na primeira lista." >&2
    exit 1
  fi
  # `-x` e nao `-f`: o `-f` casa a linha de comando do proprio shell.
  echo "reinicie o painel:  pkill -x cosmic-panel"
else
  echo "ja estava no painel, ou plugins_wings nao existe"
fi
