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

Task 7: completa — pack de 108 linhas com 12 invariantes ancoradas; AGENTS.md da
raiz deixou de dizer "workspace de um crate" e passou a declarar os 2 testes que
nunca rodaram.

Minor: o spec do card #1 tem o mesmo defeito de formato que este teve — as linhas
de "Arquivos no escopo" misturam caminho e explicacao, e o /sdd-validate usa a
linha inteira como padrao. La passou despercebido porque o diff contra master,
estando em master, era vazio.

Review (review-20260911-1230.md): 11 achados, 2 deles 🔴. Todos aplicados.

Os dois 🔴 eram de execucao e nenhum teste pegaria, porque ambos vivem na camada
que o fake substitui:
- o fd do hidraw era bloqueante e o AsyncFd exige o contrario. Reproduzido pelo
  revisor com pipe bloqueante: depois do primeiro pacote a prontidao fica em
  cache e a leitura estaciona a thread do runtime. Corrigido com O_NONBLOCK, e a
  escrita passou a esperar WRITABLE.
- prontidao falsa no monitor encerrava o Stream para sempre: SocketIter::next
  devolve None em EAGAIN, e eu lia isso como fim. Agora o laco limpa e espera.

O achado de seguranca que eu nao tinha cogitado: TOCTOU entre identificar e
abrir. O minor do hidraw e reciclado, e o laco de hotplug e a janela exata —
a consulta de bateria poderia ir para o token FIDO que herdou o numero. A
descoberta passou a devolver o rdev, conferido depois do open.

Qualidade mostrou que meus testes nao podiam falhar pelo motivo que anunciavam:
apagando o com_feature do teste de firmware ele continuava passando. Corrigido, e
quatro testes que exercitavam o dublê foram removidos. Suite 20 -> 16 no crate,
com mais forca: 37 no workspace.

Minor: identificadores internos ainda misturam portugues e ingles. A superficie
publica foi para o ingles, que e o que o card #3 consome.
Minor: events() retem thread, fd O_RDWR e socket do monitor se o Stream cair.
Minor: a codificacao do ioctl e x86/ARM; em ppc/mips _IOC_SIZEBITS difere.
