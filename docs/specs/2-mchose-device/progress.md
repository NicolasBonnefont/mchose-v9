# Progress — 2 mchose-device

Task 1: completa — segundo membro do workspace, com tokio, futures, udev e libc.
`deny(unsafe_code)` em vez de `forbid`: os ioctls de feature nao tem involucro
seguro na std e o transporte real vai precisar abrir a excecao num modulo so.

Task 2: completa (3b7e9e4) — 5 testes sobre sysfs falso mais 1 `#[ignore]` contra o
sysfs real, que achou /dev/hidraw5 (confere com o lsusb). Identificacao pelo
uevent do barramento HID: sem abrir descritor, e funciona para uhid.

Tasks 3 e 4: completas (d1b668f) — 14 testes novos, suite 19/19 mais 1 ignorada.
Emendadas num commit so: a verificacao que escrevi para a task 3 era
inalcancavel isolada, porque um trait sem consumidor e dead code e reprova em
`-D warnings`. Erro da decomposicao, nao do codigo.

Um teste derrubou um erro de design: eu tratava "canal de consulta fechado" como
fim de sessao, o que faria um consumidor que so escuta — um binario de terminal —
nunca receber evento nenhum. Canal fechado agora significa "nunca havera pedido".
