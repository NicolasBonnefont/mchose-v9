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
# O `--` e obrigatorio: sem ele o cargo le `-3` como opcao dele e aborta,
# e o comando do README (com ganho negativo) nunca funcionaria.
cargo run --locked -q -p mchose-audio --example eq -- "$@"

echo "reiniciando o filter-chain..."
systemctl --user restart filter-chain.service

# Espera com prazo, em vez de um `sleep` fixo: o serviço leva alguns segundos
# para subir, e um sleep curto reportava falha numa operacao que dera certo.
for _ in $(seq 1 30); do
  # `grep` sem `-q`, de proposito: com `-q` ele sai na primeira ocorrencia e
  # fecha o pipe, o `pw-dump` leva SIGPIPE, e sob `set -o pipefail` o pipeline
  # inteiro conta como falha — mesmo tendo encontrado. Isso fazia a checagem
  # nunca reconhecer um sink que existia.
  if pw-dump 2>/dev/null | grep mchose_v9_eq >/dev/null 2>&1; then
    echo "pronto: o sink 'MCHOSE V9 PRO EQ' existe."
    echo "Selecione-o como saida para ouvir o EQ."
    exit 0
  fi
  sleep 1
done

echo "o sink nao apareceu em 30s. Veja:  journalctl --user -u filter-chain -n 30" >&2
exit 1
