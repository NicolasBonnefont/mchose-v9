#!/usr/bin/env bash
# Instala o EQ do MCHOSE V9 PRO. Nao precisa de root.
#
# Escrever o arquivo NAO cria o sink: ele so nasce depois do restart do servico,
# e um texto recusado vira aviso no log em vez de erro. Por isso o script confere
# o no no fim, em vez de assumir.
set -euo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$RAIZ"

echo "gerando e instalando a configuracao..."
cargo run --locked -q -p mchose-audio --example eq "$@"

echo "reiniciando o filter-chain..."
systemctl --user restart filter-chain.service

sleep 2
if pw-dump 2>/dev/null | grep -q mchose_v9_eq; then
  echo "pronto: o sink 'MCHOSE V9 PRO EQ' existe."
  echo "Selecione-o como saida para ouvir o EQ."
else
  echo "o sink nao apareceu. Veja:  journalctl --user -u filter-chain -n 30" >&2
  exit 1
fi
