# mchose-v9 — contexto do repositório

Applet COSMIC que mostra bateria e controla o headset MCHOSE V9 PRO no Pop!_OS.

## Stack real

**O repositório ainda não tem código.** Em 11/09/2026 ele contém sete
arquivos: as specs, o probe em Python que validou o protocolo, a regra udev e o
`.gitignore`. Não existe manifesto de dependência.

A stack **decidida** (spec aprovada, ainda não materializada) é Rust +
`libcosmic`, em workspace de quatro crates. A decisão foi tomada contra a
alternativa Python/GTK do projeto irmão `g5-control`, que resolve a mesma classe
de problema nesta máquina — a troca foi deliberada, por causa do popover nativo
e dos sliders do EQ.

Ambiente verificado: COSMIC 1.0.0 (`cosmic-comp` a830784), `cosmic-applets`
1.0.15, PipeWire 1.6.8, Python 3.12.3, kernel 7.1.5-76070105-generic.

**Rust não está instalado nesta máquina.** Usar `rustup`; o `rustc` 1.75 do apt
é velho demais para libcosmic.

Faltam headers de build: `libudev-dev` e os de PipeWire (`libpipewire-0.3` não
está no `pkg-config`). O módulo `libpipewire-module-filter-chain.so` está
presente, que é o que o EQ vai usar em runtime.

## Comandos

| Ação | Comando |
| --- | --- |
| Testar o protocolo no hardware | `sudo python3 spikes/battery_probe.py` |
| Build | não existe ainda — `cargo build` depois do scaffold |
| Teste | **não existe ainda** |
| Lint | não existe ainda |

O probe é o único comando que funciona hoje, e funciona: leu 70% descarregando,
firmware `0012` no dongle e `0036` no fone.

## Rede de segurança automatizada

**Não existe. Zero testes, zero cobertura, zero CI, zero hook de pre-commit.**

Nada pode ser verificado automaticamente neste repositório hoje. Todo o
conhecimento validado veio de execução manual do probe contra o hardware.

O que a spec planeja, e ainda não existe:

- testes de unidade em `mchose-protocol` com os bytes reais capturados
  (`55 65 46 02` e `aa 01 00 00 01 02 ff 25`), não fixtures inventadas
- testes de I/O contra um V9 PRO virtual via `uhid`

**Ressalva sobre o `uhid`:** `/dev/uhid` existe mas é `crw------- root root`, e
o módulo não está carregado. Testes baseados nele vão exigir root ou regra udev
própria — a spec afirmou que rodariam "em CI sem o fone plugado" sem registrar
esse custo. Resolver antes de depender da estratégia.

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

**udev/logind.** `install/99-mchose-v9.rules` usa `TAG+="uaccess"` para entregar
o `hidraw` ao usuário da sessão. Requer instalação com root e replug do dongle.

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

Specs de design em `docs/superpowers/specs/AAAA-MM-DD-<tema>-design.md`.
Specs do ciclo sdd em `docs/specs/`. Spikes descartáveis em `spikes/`.

Commits sem prefixo de tipo; assunto em uma linha, corpo explicando a decisão.

## Concentrações de risco

**O protocolo é conhecimento frágil e mal distribuído.** Ele foi extraído de um
bundle JavaScript cujos hashes rotacionam e que já truncou silenciosamente uma
vez durante a investigação. A spec é o único registro dos bytes; se ela se
perder, o trabalho se perde. Tratar `docs/superpowers/specs/` como código.

**O EQ nunca teve spike.** Bateria está validada ponta a ponta; a filter-chain
do PipeWire é papel. É o item com maior chance de estourar estimativa, e por
isso é o último na ordem de construção.

**Dois recursos são inalcançáveis, não adiados:** volume dos avisos sonoros e
auto-desligamento. No Windows passam pela ConfLib da C-Media (`PropertyControl`
→ driver de kernel), sem equivalente no Linux, e **não há máquina Windows
disponível** (confirmado em 11/09/2026) para capturar o tráfego USB. Restam as
DLLs e o `.sys` em `~/Downloads/MCHOSE HUB/`, que permitiriam reversão estática
— caro e sem garantia. Não prometer esses recursos.

**Churn de API do libcosmic.** Fixar versão; não perseguir `main`.
