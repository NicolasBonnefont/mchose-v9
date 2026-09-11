# mchose-device — Module Pack

Descobre o MCHOSE V9 PRO, mantém um fluxo de eventos vivo enquanto o dongle
existir, e sobrevive a ele sumir e voltar.

## 1. Fronteira

**É dono de:** achar o dispositivo, abrir o descritor, conduzir a sessão,
sequenciar consulta e resposta, e reatar depois de um sumiço.

**Não é dono de, apesar de parecer:**

- O formato dos pacotes e a regra de quem é o V9 PRO — é o `mchose-protocol`.
  Aqui só se passa VID/PID e nome adiante (`src/discovery.rs:67`).
- A `Subscription` do iced e qualquer coisa de UI. O crate expõe um `Stream` e
  ignora a existência do libcosmic; quem embrulha é o applet, que é onde os
  applets oficiais do COSMIC põem essa função.
- Decidir o que mostrar. "Dormindo com o último valor conhecido" e "instale a
  regra udev" são frases do applet — aqui só se publica o estado.
- Instalar `install/99-mchose-v9.rules`. É operação de máquina.

## 2. Regras de negócio e invariantes

- **A identificação sai do `uevent` do barramento HID, sem abrir descritor.**
  `HID_ID` e `HID_NAME` existem para todo dispositivo HID em qualquer
  barramento — USB, I2C ou `uhid` — enquanto os atributos USB não existem no
  nível do `hidraw` — `src/discovery.rs:29`
- **Nada é aberto antes de `is_supported` devolver verdadeiro.**
  `/dev/hidraw*` inclui teclado, token FIDO e leitor biométrico; abrir para
  perguntar quem é seria tocar dispositivo alheio — `src/discovery.rs:67`
- **A escolha não depende da ordem do diretório**, que o kernel não garante —
  `src/discovery.rs:94`
- **O trait de transporte é `pub(crate)`** e nenhuma função pública aceita
  `&[u8]` rumo ao dispositivo. Um transporte público de bytes crus anularia a
  API fechada do protocolo e alcançaria `0xED` e `0x41` — `src/transport.rs:15`
- **São quatro operações, não duas.** O report `0xAA` é Feature-only no
  descritor; sem `GET_FEATURE`/`SET_FEATURE` não há firmware —
  `src/transport.rs:23`
- **`unsafe` vive num módulo só.** O crate é `deny(unsafe_code)`; a exceção é o
  transporte real, porque os ioctls de feature não têm invólucro seguro —
  `src/transport/hidraw.rs:6`
- **O `rdev` amarra identificação e uso ao mesmo dispositivo.** O minor do
  hidraw é reciclado pelo kernel, e entre achar e abrir cabe uma reenumeração —
  o laço de hotplug é justamente essa janela. Confiar no nome faria a consulta
  de bateria ir para o token FIDO que herdou o número — `src/hotplug.rs:82`
- **O descritor é aberto em modo não-bloqueante.** `AsyncFd` exige isso: é o
  `WouldBlock` da leitura que lhe diz que a prontidão era falsa. Sem
  `O_NONBLOCK`, a prontidão fica em cache depois do primeiro pacote e a leitura
  seguinte estaciona a thread do runtime — `src/transport/hidraw.rs:45`
- **Prontidão falsa no monitor não encerra nada.** `SocketIter::next` devolve
  `None` em EAGAIN e em evento reprovado no filtro; tratar isso como fim mataria
  o `Stream` para sempre — `src/hotplug.rs:128`
- **`EACCES` é estado próprio, não "sem dongle".** Sem a regra udev instalada,
  `/dev/hidraw*` fica `crw------- root root`, e o applet precisa poder dizer
  *instale a regra* — `src/machine.rs:45`
- **Canal de consulta fechado não encerra a sessão.** Um consumidor que só
  escuta não segura alça nenhuma; tratar como fim o deixaria sem evento —
  `src/machine.rs:105`
- **A ordem do `select` é determinística.** `biased` dá prioridade à leitura
  pendente sobre prazo e sobre pedido novo — `src/machine.rs:128`
- **Prazo ausente é um futuro que nunca resolve** (`src/machine.rs:117`), e não
  um segundo `select`. Duas cópias da mesma máquina davam dois lugares para
  editar e faziam o prazo reiniciar a cada volta do laço.
- **`refresh()` é síncrono e nunca espera.** Sem sessão ninguém drena o canal;
  um `send` assíncrono penduraria o applet assim que a fila enchesse —
  `src/lib.rs:47`
- **O monitor nasce antes da primeira enumeração.** Um dispositivo já plugado
  precisa disparar a mesma consulta que uma chegada — `src/hotplug.rs:55`
- **O evento é owned.** `NoReading` empresta o buffer de leitura, e o applet
  precisa de `'static` para virar `Message` — `src/machine.rs:19`

## 3. Padrão canônico

**Arquivo de referência: `src/machine.rs`.** O laço abandona a leitura em
andamento para atender um pedido — seguro, porque nada foi consumido do
descritor — e o empréstimo do transporte termina no fim do bloco `select`,
liberando-o para a ação logo abaixo. Quem mexer aqui mantém essa forma: emprestar
dentro do bloco, agir fora dele.

Módulo por assunto, testes no fim do próprio arquivo, fixtures sendo bytes
capturados do hardware com a data no comentário. Documentação e comentário em
português, identificadores em inglês.

**A regra vale, o crate ainda não cumpre por inteiro.** A superfície pública está
em inglês — `DeviceEvent`, `Demand`, `events`, `refresh`, `PermissionDenied {
path }` — porque é o que o card #3 consome e renomear depois sairia caro. Os
identificadores internos ainda misturam (`identifica`, `RAIZ_SYSFS`, `prazo`,
`achado`). Dívida registrada, não convenção nova.

O fake encena o **transporte**, nunca o `sysfs` nem o kernel. Quando faltar
cobertura, a pergunta é "que transporte produz isso?", não "como simulo o
kernel?".

## 4. Blast radius

**Consumidores diretos:** nenhum ainda. O previsto é o applet do card #3, que
consome `events()`, guarda a `Demand` e converte `DeviceEvent` em `Message`.

**Contratos que atravessam processo:** `/dev/hidraw*` e o socket do monitor do
kernel. Nenhuma rede, IPC, banco ou fila.

**Pontos de registro:** `src/lib.rs` — módulo novo exige `pub(crate) mod` lá, e
é onde a superfície pública é decidida. `Cargo.toml` da raiz lista os membros.

**Onde mora a condicional:** `aguardando` decide se há prazo de resposta
(`src/machine.rs:100`) e `ha_quem_peca` se o canal de consulta ainda vale
(`:103`). Não há flag de configuração nem variação por cliente.

## 5. Armadilhas

**A supervisão roda em thread própria, e não é preferência.**
`udev::MonitorSocket` guarda ponteiros crus e não é `Send`: não atravessa thread
e não entra numa task compartilhada. Confinar o socket numa thread com runtime
`current_thread` é o que torna o resto possível — `src/lib.rs:59`. Quem
"simplificar" isso para um `tokio::spawn` vai bater no mesmo muro.

**`events()` deixa a thread viva se o `Stream` for descartado** — e com ela o fd
`O_RDWR` do HID e o socket do monitor. Um applet chama uma vez e vive enquanto o
painel vive, então na prática não aparece; chamar N vezes multiplica os três.
Dívida consciente, registrada no ledger do card.

**Os testes aqui provam a lógica, não o dublê.** Quatro testes de
`transport.rs` foram removidos numa revisão por exercitarem o fake em vez do
código. Sobraram os dois que fixam convenção não óbvia: silêncio pende para
sempre, sumiço é `NotConnected`. Teste novo que só confirme o que o fake faz não
volta.

**Os testes do caminho real nunca rodaram.** Os dois `#[ignore]` de
`src/transport/hidraw.rs` dependem de acesso ao `hidraw`, e a regra udev não
está instalada nesta máquina. O que está provado é a lógica sobre o fake; a
camada de ioctl é papel até alguém rodar.

**Compilar nunca acontece com privilégio.** `cargo test` executa `build.rs` e
proc-macros de toda a árvore; sob `sudo`, um PR que acrescente dependência vira
root. Compila-se com `--no-run` e só o binário pronto roda com privilégio.

**`/dev/uhid` não recebe regra udev.** Dar `uaccess` nele permite criar teclado
HID virtual e injetar entrada na sessão — escalada local, não conveniência.
