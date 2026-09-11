# 2 — mchose-device: acesso ao hidraw com hotplug

**Card:** https://github.com/NicolasBonnefont/mchose-v9/issues/2
**Módulo:** `crates/mchose-device`
**Status:** rascunho

## Comportamento atual

`crates/mchose-device` não existe. O workspace tem um membro só,
`crates/mchose-protocol`, que converte bytes em tipos e **não abre arquivo
nenhum** — ninguém no repositório fala com o dispositivo em Rust.

O único código que fala com o dispositivo é `spikes/battery_probe.py`, e ele
diverge do que este card precisa em quatro pontos:

- `spikes/battery_probe.py:29` casa **só por VID/PID**. É o defeito que o crate
  de protocolo corrigiu (`crates/mchose-protocol/src/device.rs:30`) e que este
  card não pode reintroduzir ao enumerar.
- `spikes/battery_probe.py:22` abre **todo** `/dev/hidraw*` com `O_RDWR` só para
  perguntar quem é, inclusive teclado e mouse de outros fabricantes.
- a leitura é um disparo único com `select` e prazo fixo; não há laço, nem
  máquina de estados, nem escuta contínua do push.
- não há hotplug: o dongle sumir encerra o processo.

## Comportamento alvo

Um crate `mchose-device` que descobre o V9 PRO, mantém um fluxo de eventos vivo
enquanto o dongle existir, e sobrevive a ele sumir e voltar.

**Descoberta.** Lê VID/PID e nome do produto de
`/sys/class/hidraw/<n>/device/uevent` — os campos `HID_ID` e `HID_NAME` —
**sem abrir descritor nenhum**, e entrega os dois a
`mchose_protocol::device::is_supported`. A decisão de quem é o V9 PRO **não é
reimplementada aqui**.

Essa fonte foi escolhida contra as duas alternativas: os atributos USB
(`ATTRS{idVendor}`) não existem no nível do `hidraw` e somem num dispositivo
virtual sem pai USB, o que quebraria os testes; e o ioctl exigiria abrir o
descritor de todo `hidraw` da máquina antes de saber de quem é. O `uevent` do
barramento HID existe para qualquer dispositivo HID, seja USB, I2C ou `uhid`,
e o `HID_NAME` já vem na forma longa que o ioctl devolve.

**Transporte atrás de um trait.** O laço de leitura, a máquina de estados e a
reconexão operam sobre uma abstração de transporte. A implementação real fala
com o `hidraw`; em teste, um fake devolve os pacotes capturados do hardware e
encena dongle sumindo, resposta truncada e silêncio.

O trait é **um só**, `pub(crate)`, com **quatro** operações: ler, escrever,
`GET_FEATURE` e `SET_FEATURE`. As duas últimas não são luxo — o report `0xAA` é
declarado **Feature-only** no descritor (página `0xFF22`), e só o `0x55` tem
Input/Output, então um transporte de apenas ler e escrever não alcançaria o
firmware que este card promete. Enumeração e monitor de chegada ficam **fora**
do trait, em código concreto: o fake encena o transporte, nunca o `sysfs`.

A espera de `FEATURE_ROUNDTRIP_WAIT` entre escrever e ler o feature é
assíncrona. Dormir bloqueando trava o painel do card #3.

**Fluxo de eventos.** O crate expõe um `Stream` assíncrono de eventos
**owned**. Não conhece `iced` nem `libcosmic`: quem embrulha em
`Subscription::run_with` com `iced::stream::channel` é o applet do card #3, que
é onde os applets oficiais do COSMIC põem essa função
(`cosmic-applet-bluetooth/src/bluetooth.rs`).

**Firmware na conexão.** Ao descobrir o dongle, o crate consulta as duas versões
— dongle e fone — e as publica num evento de conexão, que o card #3 guarda. Isso
custa duas esperas de `FEATURE_ROUNDTRIP_WAIT` antes do primeiro número de
bateria, e a alça de consulta fica com uma operação só.

**Escuta passiva.** O fone empurra a atualização de bateria sozinho. O crate
consulta em **dois momentos, e só neles**: quando o dongle aparece, e quando o
consumidor pede explicitamente. Não há timer, não há polling.

A alça de consulta sob demanda é `Clone + Send`, obtida fora do `Stream`, e
pedir com o dongle ausente é no-op — nunca erro que encerre o fluxo.

**Hotplug.** O crate percebe o dispositivo aparecer e sumir por notificação do
kernel, sem varrer diretório em laço. A notificação apenas **sinaliza**: a
identificação vem sempre do `sysfs`, nunca dos atributos do evento.

O monitor é criado **antes** da enumeração inicial, e o dispositivo já presente
nessa enumeração dispara a mesma consulta que uma chegada dispararia — senão o
applet iniciado com o dongle plugado nunca consulta, e fica esperando um push
que pode não vir.

Perder o dongle não encerra o fluxo nem derruba o consumidor: o crate volta a
esperar e reata quando ele voltar.

**Estados observáveis.** O consumidor recebe transições, não adivinha:

| Estado | Origem |
| --- | --- |
| sem dongle | nenhum `hidraw` casa |
| sem permissão | abrir devolveu `EACCES` |
| dongle sem resposta | consulta feita, nada em `BATTERY_RESPONSE_TIMEOUT` |
| leitura | o `BatteryReading` como o protocolo devolveu |

**`EACCES` não é "sem dongle".** Verificado em 11/09/2026: `/dev/hidraw5` está
`crw------- root root`, sem ACL, porque `install/99-mchose-v9.rules` não foi
instalada — hoje esse é o caminho normal, não a exceção. O card #3 precisa poder
dizer *instale a regra*, e não *fone desligado*.

**Dormir não é estado deste crate.** `ChargeState::Asleep` chega dentro da
leitura; promovê-lo a estado irmão criaria dois caminhos para o mesmo byte e
faria o card #3 tratar sono duas vezes.

**Desaparecimento nunca é "desligado".** O fone se desconecta do dongle 2.4GHz
quando é plugado para carregar, então sumir e estar desligado são
indistinguíveis daqui. O crate reporta ausência; quem redige a frase é o
card #3, e o vocabulário do evento não pode induzi-lo ao erro.

## Invariantes

- **O crate não conhece `iced` nem `libcosmic`.** É o que permite usá-lo num
  binário de terminal ou num daemon sem arrastar a UI
  (`AGENTS.md`, seção Arquitetura).
- **Valem as invariantes do `crates/mchose-protocol/AGENTS.md`,** seções
  Fronteira e Regras de negócio: identificação delegada, nome como bytes, e
  nenhuma entrada causa pânico. O nome trafega como bytes até o protocolo —
  `to_string_lossy` reintroduziria o defeito do padding.
- **O evento de falha carrega uma cópia dos bytes rejeitados** — os 4 do canal
  de bateria ou os 6 do de firmware que `NoReading.rejected` já delimita, nunca
  o buffer inteiro. Sem isso o byte estranho morre dentro do crate e
  `ChargeState::Unknown` perde o propósito: o protocolo deixa de ser mapeável.
- **O evento publicado é owned e `Clone + Send + 'static`.** `NoReading<'a>`
  empresta o buffer de leitura (`crates/mchose-protocol/src/lib.rs:33`), então
  nada que o carregue pode ser `'static`, e o card #3 precisa convertê-lo em
  `Message` do iced.
- **O trait de transporte é `pub(crate)`.** Nenhuma função pública do crate
  aceita `&[u8]` rumo ao dispositivo: a escrita só recebe os requests que o
  `mchose-protocol` constrói, e apenas os dois de consulta. Um transporte
  público de bytes crus anularia a API fechada do card #1 e voltaria a alcançar
  `0xED` e `0x41`.
- **Um único dono do descritor.** Um fd aberto por dispositivo; nada de abrir
  para perguntar e reabrir para ler.
- **Nada é aberto antes de `is_supported` devolver verdadeiro.** `/dev/hidraw*`
  inclui teclado, token FIDO e leitor biométrico. VID/PID e nome vêm do `sysfs`;
  nenhum `read`, `write` ou ioctl de feature acontece em dispositivo alheio, e
  `O_RDWR` só depois de identificado. O probe abre todos com `O_RDWR` só para
  perguntar quem é — este card não herda isso.
- **O crate nunca registra dado de dispositivo alheio:** nem nome, nem serial
  (`HID_UNIQ`), nem bytes. Do nosso, só o que `NoReading.rejected` já delimita.
- **O fluxo não termina porque o dongle sumiu.** O laço de reconexão é o padrão
  dos applets do COSMIC: estado, espera, reentra. Fim de stream é fim de applet.
- **Nenhum tráfego em estado estacionário.** Consulta só na conexão e sob
  demanda; o resto é escuta.
- **Nenhuma entrada causa pânico.** Mesma disciplina do crate de protocolo:
  acesso verificado, sem `unwrap`/`expect`/`panic!` no código de produção.

## Fora de escopo

- A função de `subscription` do iced e qualquer coisa de UI — card #3.
- Escrever configuração no dispositivo. Só os dois requests de consulta que o
  crate de protocolo constrói.
- EQ, surround, áudio — card #4.
- Instalar a regra udev. `install/99-mchose-v9.rules` já existe; instalar é
  operação de máquina, não código deste card.
- **Versionar qualquer regra udev nova.** Em particular, `/dev/uhid` não recebe
  `uaccess` nem `MODE=0666`: dar `uhid` ao usuário permite a qualquer processo
  da sessão criar um teclado HID virtual e injetar entrada no compositor — é
  escalada local, não conveniência de teste. `install/99-mchose-v9.rules` fica
  intocado.
- Reexportar a API do `mchose-protocol`. Quem precisa dos tipos depende do crate
  de protocolo diretamente.
- Suporte aos irmãos que dividem o VID/PID. Decidido no card #1: só V9 PRO.
- CI. Continua fora, como no card #1.

## Arquivos no escopo

- `Cargo.toml`
- `Cargo.lock`
- `crates/mchose-device/**`
- `AGENTS.md`
- `docs/specs/2-mchose-device/**`

O `Cargo.toml` da raiz acrescenta `crates/mchose-device` aos membros, e o
`Cargo.lock`, versionado, muda na primeira dependência externa do projeto. No
`AGENTS.md` da raiz, *Stack real* ("workspace de um crate" e "faltam headers"
ficam falsos), *Comandos* e *Rede de segurança automatizada*. O
`crates/mchose-device/AGENTS.md` é o Module Pack que o card #3 vai ler.

Dentro do crate, módulo por assunto, como no card #1: `discovery` (sysfs),
`transport` (o trait, o fake e a implementação sobre hidraw), `machine` (a
sessão e seus estados), `hotplug` (a supervisão que atravessa as sessões) e
`lib` (o `Stream`, a alça e o evento).

O `hotplug` não constava da primeira redação desta lista — a supervisão estava
diluída entre `machine` e `lib`, e separá-la só ficou evidente na implementação,
quando a sessão e o que atravessa sessões se mostraram dois assuntos.

## Critério de verificação

```bash
cargo test -p mchose-device
cargo clippy -p mchose-device -- -D warnings -D clippy::indexing_slicing \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
cargo fmt --check
```

Sobre o fake de transporte, sem privilégio nenhum:

- o pacote real `55 65 46 02` vira evento de leitura válida
- push espontâneo, sem consulta anterior, também vira evento
- consulta sem resposta em `BATTERY_RESPONSE_TIMEOUT` vira "dongle sem resposta"
- tráfego alheio (report `0x06`) não produz evento nem registro
- transporte que some no meio de uma leitura não encerra o fluxo
- transporte que volta produz nova leitura sem o consumidor reassinar nada
- nenhuma consulta é emitida em estado estacionário
- consulta sob demanda, com o fluxo já em estado estacionário, produz leitura
- pedir consulta com o dongle ausente é no-op e não encerra o fluxo
- `EACCES` ao abrir produz o estado de permissão, e não "sem dongle"

Caminho real, sob demanda e com privilégio:

**Compilar nunca acontece com privilégio.** `cargo test` executa `build.rs` e
proc-macros de toda a árvore de dependências; sob `sudo` isso é root para
qualquer dependência que um PR acrescente, num repositório público. Compila-se
sem privilégio e só o binário pronto roda com ele:

```bash
sudo modprobe uhid
cargo test -p mchose-device --no-run          # sem sudo
sudo ./target/debug/deps/<binario> --ignored  # só o que ja esta compilado
```

`sudo -E` é proibido: preservar o ambiente do usuário sob root expõe
`CARGO_HOME` e `RUSTC_WRAPPER` a escrita de terceiros.

- um V9 PRO virtual criado com `uhid` é descoberto e responde
- um dispositivo virtual com o mesmo VID/PID e nome `MCHOSE S9 PRO` **não** é
  descoberto
- o nome na forma curta (`MCHOSE V9 PRO`, sem o prefixo do fabricante) é aceito
- um dispositivo de VID/PID alheio é enumerado sem nunca ser aberto

Verificação manual contra o hardware, declarada porque a camada de `ioctl` e
`AsyncFd` não é coberta pelo fake: com o dongle plugado, um binário de exemplo
imprime a bateria e continua imprimindo quando o dongle é removido e recolocado.
