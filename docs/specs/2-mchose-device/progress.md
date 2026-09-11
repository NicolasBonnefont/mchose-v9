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

Tasks 3 a 6: completas (d8e7256) — 20 testes no crate, 41 no workspace, 2 ignorados
(caminho real). Emendadas num commit so: a decomposicao errou ao supor que trait,
maquina de estados e transporte podiam ser verificados isolados. Trait sem
consumidor e dead code, e `run_session` sem o Stream publico tambem — o portao
`-D warnings` so fecha quando o ponto de entrada existe.

Tres correcoes que os testes e o compilador forcaram, nao a revisao:
- canal de consulta fechado nao encerra sessao; um consumidor que so escuta nao
  segura alca nenhuma e ficaria sem evento.
- `run_session` publico com trait `pub(crate)` nao compila. A invariante de
  seguranca fechou a superficie sozinha: so `DeviceEvent`, `Demand` e `events`.
- `udev::MonitorSocket` nao e `Send`. A supervisao roda em thread propria com
  runtime current_thread, e o canal de eventos atravessa de volta.

Minor: `events()` deixa a thread viva se o Stream for descartado; um applet
chama uma vez, mas nao ha desligamento explicito.
Minor: os dois testes `#[ignore]` do caminho real nunca rodaram — /dev/hidraw5
esta root-only porque a regra udev nao foi instalada nesta maquina.
