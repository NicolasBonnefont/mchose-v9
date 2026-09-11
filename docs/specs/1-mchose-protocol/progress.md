# Progress — 1 mchose-protocol

Task 1: completa — workspace, toolchain 1.98.1 fixada, `forbid(unsafe_code)` e os
quatro lints antipanico ativos. Sem testes: e scaffold, a verificacao e clippy + fmt.

Task 2: completa (7e8f070) — 5 testes novos, suite 5/5. Cobre as duas formas do
nome, V9 PRO 2, os irmaos de mesmo VID/PID, nome vazio e 4096 bytes de lixo.

Task 3: completa (6680601) — 3 testes novos, suite 8/8. Constantes de 300 ms e 2 s
com procedencia declarada; API de request fechada, sem construtor generico.

Task 4: completa (bde67f7) — 8 testes novos, suite 16/16. Um tipo so de falha,
carregando os bytes quando o pacote e do canal de bateria e None quando e
trafego alheio (tecla de midia, telefonia) que chega no mesmo /dev/hidraw.
