# mchose-v9 — contexto do repositório

Applet COSMIC que mostra bateria e controla o headset MCHOSE V9 PRO no Pop!_OS.

## Stack real

Workspace Rust de **três** crates: `crates/mchose-protocol`, puro e sem I/O
(card #1); `crates/mchose-device`, que fala com o `/dev/hidraw` e expõe um
`Stream` de eventos (card #2); e `cosmic-applet-mchose`, o applet do painel
(card #3). Falta `mchose-audio`, card #4.

O libcosmic entra fixado no rev `d4d71fd5`, o mesmo que os `cosmic-applets`
1.0.15 desta máquina usam. **A API dele se lê na fonte do checkout, não no doc:**
o doc manda declarar `cosmic = { version = "1.0" }` do crates.io, que é um build
tool de C/C++ sem relação com o projeto.

A stack foi escolhida contra a alternativa Python/GTK do projeto irmão
`g5-control`, que resolve a mesma classe de problema nesta máquina. A troca foi
deliberada, por causa do popover nativo e dos sliders do EQ.

Toolchain fixada em `rust-toolchain.toml` (1.98.1) e instalada via `rustup`; o
`rustc` 1.75 do apt não serve. `Cargo.lock` é versionado.

O crate de protocolo é **sem dependências externas**, de propósito. O de
dispositivo traz `tokio`, `futures`, `udev` e `libc` — e é o único lugar do
projeto com `unsafe`, confinado ao módulo dos ioctls.

Ambiente verificado: COSMIC 1.0.0 (`cosmic-comp` a830784), `cosmic-applets`
1.0.15, PipeWire 1.6.8, Python 3.12.3, kernel 7.1.5-76070105-generic.

**Rust não está instalado nesta máquina.** Usar `rustup`; o `rustc` 1.75 do apt
é velho demais para libcosmic.

Headers de build instalados: `libudev` 255 e `libpipewire-0.3` 1.6.8, mais as
dezoito bibliotecas que o libcosmic exige. O módulo
`libpipewire-module-filter-chain.so` está presente, e o sistema traz templates
prontos em `/usr/share/pipewire/filter-chain/`.

## Comandos

| Ação | Comando |
| --- | --- |
| Teste | `cargo test --locked` |
| Lint | `cargo clippy --workspace --locked -- -D warnings -D clippy::indexing_slicing -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` |
| Formato | `cargo fmt --check` |
| Build | `cargo build --locked` |
| Teste do caminho real | `cargo test --locked -p mchose-device --no-run` e então o binário sob privilégio com `--ignored` |
| Testar o protocolo no hardware | `python3 spikes/battery_probe.py` |

O lint **não** usa `--all-targets`, de propósito: os lints antipânico existem
para o código que recebe bytes do dispositivo, e `expect` num teste é como o
teste declara falha.

O probe precisa de `sudo` **enquanto** `install/72-mchose-v9.rules` não estiver
instalada; com a regra aplicada e o dongle replugado, o `uaccess` entrega o
`hidraw` ao usuário da sessão e o `sudo` deixa de ser necessário. Rodar como root
um script que faz `O_RDWR` em canal vendor de firmware é privilégio a mais.

O probe leu 70% descarregando, firmware `0012` no dongle e `0036` no fone — é a
prova de que os bytes das fixtures vieram do hardware.

## Rede de segurança automatizada

**47 testes de unidade, todos rodando sem hardware e sem privilégio** — 21 em
`mchose-protocol`, 16 em `mchose-device` e 10 no applet. As fixtures são bytes reais
capturados do dispositivo (`55 65 46 02`, `aa 01 00 00 01 02 ff 25`), não
inventadas.

**Mais 2 marcados `#[ignore]`**, que exercitam o ioctl e o `AsyncFd` contra o
hardware. Rodaram em 11/09/2026 com a regra udev instalada, **sem privilégio**:
descoberta em `/dev/hidraw5` e leitura de 70% descarregando. Ficam sob demanda
porque dependem do dongle plugado.

**`--locked` não é zelo.** Sete dependências vêm de branch móvel do `pop-os` sem
commit fixado no manifesto, incluindo o `cosmic-config-derive`, que é proc-macro
e executa em tempo de compilação. Só o `Cargo.lock` fecha a árvore.

**Compilar nunca acontece com privilégio.** `cargo test` executa `build.rs` e
proc-macros de toda a árvore; sob `sudo`, um PR que acrescente dependência vira
root na máquina de quem revisar.

**Zero CI e zero hook de pre-commit** — os três comandos acima rodam à mão. O
crate de protocolo é o único do projeto que roda sem hardware, então é o
candidato natural quando houver um card de CI.

Ainda não existe: um V9 PRO virtual via `uhid` para o caminho real.

**Ressalva sobre o `uhid`:** `/dev/uhid` existe mas é `crw------- root root`, e
o módulo não está carregado. Testes baseados nele exigem root — e **não** se
resolve com regra udev: dar `uaccess` em `/dev/uhid` permite a qualquer processo
da sessão criar teclado HID virtual e injetar entrada no compositor. É escalada
de privilégio local, não conveniência de teste. A estratégia é fake em trait
para a lógica, e testes `#[ignore]` rodados sob demanda para o caminho real.

## Arquitetura

Planejada, não construída. Binário único: o applet é dono do `hidraw`, sem
daemon separado — um applet de painel vive enquanto o painel vive.

```
crates/mchose-protocol/   sem I/O, puro, testável sem hardware
crates/mchose-device/     hidraw + hotplug por udev
crates/mchose-audio/      EQ e surround via PipeWire filter-chain
cosmic-applet-mchose/     applet libcosmic
```

A fronteira que mais importa é `mchose-protocol` não abrir arquivo nenhum: é a
unidade que sai inteira depois, seja para um daemon, seja para um PR ao
HeadsetControl.

Referência de estilo para o que **não** é Rust (scripts de instalação, layout
de `install/`, specs em português): o projeto irmão `g5-control`, que resolve a
mesma classe de problema — hardware que o Pop!_OS não expõe — e fica fora deste
repositório.

## Fronteiras

**Hardware.** USB `291d:385d` via `/dev/hidraw*`. O VID/PID **não identifica o
produto** — é compartilhado por S9 PRO, G9 PRO, V9 e V9 PRO. Casar sempre
VID/PID **mais** nome do produto, excluindo `V9 PRO 2`, que usa outro caminho no
firmware. Errar isso faz o applet falar com o fone errado.

**PipeWire.** O EQ cria um sink virtual via `filter-chain`. Mexe na
configuração de áudio do usuário, fora do repositório.

**udev/logind.** `install/72-mchose-v9.rules` usa `TAG+="uaccess"` para entregar
o `hidraw` ao usuário da sessão. **O número do arquivo importa:** quem converte a
tag em ACL é o `73-seat-late.rules`, então uma regra acima de 73 marca o
dispositivo tarde demais e nada acontece — foi o que houve enquanto ela se
chamava `99-`. Requer instalação com root e `udevadm trigger` (ou replug).

**GitHub.** `origin` é `github.com:NicolasBonnefont/mchose-v9`, repositório
**público**; branch base é `master`. Os cards do ciclo sdd são as issues desse
repositório. Sendo público, nada de caminho absoluto com nome de usuário, dump
bruto de dispositivo ou dado pessoal no que for versionado.

## Pontos de registro

Para um recurso novo do dispositivo aparecer, hoje a lista é curta porque o
código não existe. Quando existir: comando em `mchose-protocol`, exposição do
evento em `mchose-device`, widget no popover do applet.

Para o applet aparecer no painel: desktop entry com `X-CosmicApplet=true` e
`Categories=COSMIC;`, mais o ID na lista RON de
`~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_wings`.

## Convenções

Documentação, comentários e mensagens de commit em **português**. Comentário
explica *por quê*, não *o quê* — é o padrão do `g5-control` e a spec segue ele.

No `Arquivos no escopo` de um spec, cada item precisa ser **só o caminho ou o
glob** — o `/sdd-validate` usa a linha inteira como padrão, então
`` `Cargo.toml` (raiz) — acrescenta...`` nunca casa com `Cargo.toml`. A
explicação vai em prosa, abaixo da lista.

Specs do ciclo sdd em `docs/specs/`. Spikes descartáveis em `spikes/`. O
protocolo e as armadilhas de hardware moram no `README.md`, que é público e é a
referência para quem chegar.

Commits sem prefixo de tipo; assunto em uma linha, corpo explicando a decisão.

## Concentrações de risco

**O protocolo é conhecimento frágil e mal distribuído.** Ele foi extraído de um
bundle JavaScript cujos hashes rotacionam e que já truncou silenciosamente uma
vez durante a investigação — baixar conferindo o `Content-Length`. Os registros
que sobraram são o `README.md` (bytes, famílias, armadilhas), os testes do
`mchose-protocol` (as capturas reais) e `spikes/battery_probe.py`. Tratar os três
como código: se sumirem, o trabalho se refaz do zero.

**O EQ teve spike executado antes do spec** (11/09/2026): a filter-chain cria o
sink virtual, e mudar ganho ao vivo funciona — **mas só com o sink ativo**.
Suspenso, o comando é aceito e silenciosamente ignorado. O surround ficou de
fora: depende de um HRIR que não temos e cujo licenciamento não dá para
verificar.

**Dois recursos são inalcançáveis, não adiados:** volume dos avisos sonoros e
auto-desligamento. No Windows passam pela ConfLib da C-Media (`PropertyControl`
→ driver de kernel), sem equivalente no Linux, e **não há máquina Windows
disponível** (confirmado em 11/09/2026) para capturar o tráfego USB. Restam as
DLLs e o `.sys` em `~/Downloads/MCHOSE HUB/`, que permitiriam reversão estática
— caro e sem garantia. Não prometer esses recursos.

**Churn de API do libcosmic.** Fixar versão; não perseguir `main`.
