# Progress — 1 mchose-protocol

Task 1: completa — workspace, toolchain 1.98.1 fixada, `forbid(unsafe_code)` e os
quatro lints antipanico ativos. Sem testes: e scaffold, a verificacao e clippy + fmt.

Task 2: completa (e540669) — 5 testes novos, suite 5/5. Cobre as duas formas do
nome, V9 PRO 2, os irmaos de mesmo VID/PID, nome vazio e 4096 bytes de lixo.

Task 3: completa (6680601) — 3 testes novos, suite 8/8. Constantes de 300 ms e 2 s
com procedencia declarada; API de request fechada, sem construtor generico.

Task 4: completa (7fd18f9) — 8 testes novos, suite 16/16. Um tipo so de falha,
carregando os bytes quando o pacote e do canal de bateria e None quando e
trafego alheio (tecla de midia, telefonia) que chega no mesmo /dev/hidraw.

Task 5: completa (ac92ee3) — 4 testes novos, suite 20/20. NoReading subiu para a
raiz do crate, agora que bateria e firmware compartilham a mesma falha unica.

Task 6: completa (303dbe5) — pack de 119 linhas com 12 invariantes ancoradas em
codigo; AGENTS.md da raiz deixou de afirmar que nao ha codigo, teste nem build.

Minor: repositorio publico sem LICENSE — nao estava no escopo do card, mas e
decisao pendente e afeta quem quiser reusar o protocolo.
Minor: `FirmwareTarget` nasceu exaustivo, ao contrario dos outros tipos
publicos. Sao dois alvos definidos pelo hardware (dongle e fone) e um `_` no
consumidor nao teria o que tratar.
Minor: `cargo clippy --all-targets` reprova os testes (usam `expect`). O comando
do spec nao usa a flag, de proposito; registrado no pack.

Review (review-20260911-1120.md): 9 achados, nenhum 🔴. Todos aplicados.
O mais serio foi de regressao: `str::from_utf8` validava o buffer inteiro do
ioctl, entao um V9 PRO legitimo com padding nao-UTF8 depois do NUL deixaria de
ser reconhecido em silencio. Corrigido cortando no primeiro NUL, com dois testes
novos. Suite 20 -> 21 (um teste que assertava a propria declaracao saiu, dois de
padding entraram).

Minor: `resolver` subiu de "2" para "3" — a descricao do spec dizia "2", mas
edition 2024 usa 3 por padrao e a diferenca passa a importar quando o card #2
trouxer `udev`. Desvio consciente do texto do spec, nao acomodacao.
