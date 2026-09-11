# 3 — cosmic-applet-mchose: bateria no painel

**Card:** https://github.com/NicolasBonnefont/mchose-v9/issues/3
**Módulo:** `cosmic-applet-mchose`
**Status:** rascunho

> Escrito sem entrevista, por decisão do autor de rodar os cards restantes sem
> interação. As escolhas que teriam virado pergunta estão marcadas como
> **premissa** e são reversíveis numa linha.

## Comportamento atual

Não existe applet. O workspace tem dois crates, e o `mchose-device` já entrega
tudo que este card precisa: `events()` devolve um `Stream` de `DeviceEvent` mais
uma `Demand`, e `DeviceEvent` cobre `Connected`, `Battery`, `NoResponse`,
`Rejected`, `Disconnected`, `NoDevice` e `PermissionDenied`.

O ambiente aceita applet de terceiros: o painel já roda
`com.github.hmrdsmoke.soulless-launcher.Applet`, listado em
`~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_wings`.

**Divergência encontrada na investigação:** a documentação do libcosmic manda
declarar `cosmic = { version = "1.0", features = ["applet"] }`, como se fosse um
crate publicado. Não é: o `cosmic` do crates.io é um build tool de C/C++ sem
relação com o projeto, e sua última versão é `0.1.0`. O libcosmic só existe como
dependência git.

## Comportamento alvo

Um binário `cosmic-applet-mchose` que mostra a bateria do fone no painel.

**Entrada.** Consome `mchose_device::events()`. O applet é o primeiro consumidor
real do crate, e não fala com `/dev/hidraw` nem conhece o protocolo.

**A assinatura.** O applet abre `events()` uma vez por tentativa e mantém o
fluxo vivo enquanto o processo viver. Como embrulhar o `Stream` é decisão da
implementação; os applets oficiais do COSMIC são a referência.

**O fluxo pode acabar, e o applet precisa reagir.** A invariante da primeira
redação — "a assinatura não termina" — era falsa: o `hotplug` tem um `.ok()?` que
encerra a thread em silêncio, e `events()` devolve `io::Result`. Fluxo esgotado
ou `Err` viram **estado visível de falha** e nova tentativa com espera, nunca
fim mudo. O identificador da assinatura é constante: derivá-lo de estado recriaria
a thread, o fd `O_RDWR` e o socket do monitor a cada abrir de popover.

**No painel.** Ícone mais o percentual em texto.

> **Premissa:** ícone de fone (`audio-headphones-symbolic`), não de bateria. A
> bateria do painel já é a do notebook, e um segundo ícone de bateria seria
> ambíguo justamente no lugar onde a clareza importa. Evita também depender de
> nomes de ícone por nível, que variam com o tema.

No painel há **uma regra só**: ícone fixo, mais o último percentual conhecido
quando existir. Estado e motivo vivem no popover. Variar o ícone por evento
traria de volta a dependência de nome de ícone que a premissa acima evita.

**No popover.** Estado por extenso, percentual, e as versões de firmware do
dongle e do fone que vieram no evento de conexão. Abrir o popover chama
`Demand::refresh()` — é o gatilho sob demanda que o card #2 construiu.

**Os estados, sem mentir.** Cada `DeviceEvent` tem uma frase, e nenhuma delas
inventa certeza que não existe:

São **sete** variantes em `crates/mchose-device/src/machine.rs`, e todas têm
linha:

| Evento | O que o applet diz |
| --- | --- |
| `Connected` | dongle achado; as versões vão para o popover, "desconhecida" quando `None` |
| `Battery` | percentual e estado de carga |
| `NoResponse` | "fone não respondeu — pode estar desligado ou carregando" |
| `Disconnected` | "dongle removido" |
| `NoDevice` | "dongle não encontrado" |
| `PermissionDenied` | "sem acesso ao dispositivo — instale a regra `72-mchose-v9.rules`" |
| `Rejected` | não muda a UI |

**A frase de "carregando" pertence ao `NoResponse`, não ao `Disconnected`.** Foi
uma premissa errada da primeira redação, derrubada na leitura do código:
`Disconnected` é o transporte morrendo, ou seja o dongle saindo do USB, e o
`hotplug` emite `NoDevice` logo atrás — é sempre transitório. O fone plugado para
carregar deixa o dongle vivo e o rádio mudo, o que produz `NoResponse`. Trocar os
dois faria o painel piscar "pode estar carregando" seguido de "dongle não
encontrado" toda vez que o dongle fosse removido.

`PermissionDenied` **nunca** vira "sem dongle": a diferença entre "não achei" e
"não pude abrir" é a diferença entre não ter o fone e ter instalação incompleta.

Versão de firmware já conhecida não é apagada por um `Connected` posterior que
traga `None` — o campo é `Option` justamente porque o fone às vezes não responde.

Em `ChargeState::Asleep` o percentual acompanha o pacote mas não é tratado como
autoritativo: o pack do protocolo registra que esse caso nunca foi capturado no
hardware.

**O desktop entry.** `Type=Application`, `Categories=COSMIC;`,
`NoDisplay=true`, `X-CosmicApplet=true` e `Icon=` simbólico. O `Exec=` usa
**caminho absoluto** de instalação, não um nome resolvido pelo `PATH` — o painel
lança o processo, e resolver por `PATH` deixa qual binário roda na mão do
ambiente.

**Versão do libcosmic fixada.** `rev = "d4d71fd53e5ed6bd3a430089114dffa2da3cd498"`,
que é o que os applets oficiais 1.0.15 desta máquina usam — apurado no
`Cargo.lock` deles, não escolhido pelo topo do `master`. O repositório não tem
tag `1.0`; as que existem param em `v0.12`.

## Invariantes

- **O applet não conhece o protocolo nem o `hidraw`.** Ele consome
  `DeviceEvent` e nada mais; quem fala com o dispositivo é o `mchose-device`
  (`crates/mchose-device/AGENTS.md`, seção Fronteira).
- **O estado do applet é um módulo sem libcosmic.** Ele recebe `DeviceEvent` e
  devolve o que mostrar — frase, percentual, firmware. O `Application` só traduz
  evento em `Message` e estado em widget, e não decide nada. É isso que torna o
  critério de verificação abaixo executável; sem essa fronteira, as sete frases
  cairiam dentro de um `update()` e nenhum teste rodaria.
- **Fluxo esgotado vira falha visível e nova tentativa**, nunca fim mudo. O
  identificador da assinatura é constante.
- **`DeviceEvent` e `ChargeState` são `#[non_exhaustive]`:** o braço `_` é
  obrigatório, preserva o último percentual e não muda a frase. Variante nova não
  quebra o build — a tabela de estados é o único lugar que cobra.
- **`Disconnected` e `PermissionDenied` têm frases próprias**, distintas entre si
  e distintas de "sem dongle" (`crates/mchose-device/src/machine.rs`).
- **O último percentual conhecido sobrevive a `NoResponse` e a `Disconnected`.**
  Some o estado, não o número — apagar tudo faria o painel piscar a cada
  oscilação do rádio.
- **`Demand::refresh()` é síncrono e pode ser chamado à vontade.** Sem sessão é
  no-op (`crates/mchose-device/src/lib.rs`).
- **A notificação de bateria baixa dispara por cruzamento, não por leitura.** O
  fone empurra atualização sozinho; notificar por leitura encheria a tela.
- **Nenhuma entrada causa pânico.** Mesma disciplina dos outros dois crates, com
  os mesmos lints.
- **Nem a UI nem exemplo algum imprimem bytes crus.** `Rejected` aparece só como
  contagem, e a frase de `PermissionDenied` não carrega o caminho do device.
  Nenhum caminho absoluto, nome de usuário ou conteúdo de pacote sai do applet —
  o repositório é público e é lá que log colado vai parar.
- **O applet não expõe socket, porta nem serviço D-Bus que aceite comando**, e
  nenhum argumento de linha de comando alcança o transporte. Ele é o único
  processo da sessão com o fd `O_RDWR` do canal vendor aberto; qualquer IPC o
  transformaria em proxy de escrita para o firmware.
- **O `Cargo.lock` é a fonte de verdade da árvore.** O `rev` do libcosmic fixa
  qual árvore, não quem a produziu, e não diz nada sobre os 639 crates
  transitivos, que trazem `build.rs` e proc-macros executados em tempo de
  compilação. O `rev` é copiado do `Cargo.lock` dos `cosmic-applets` 1.0.15 e só
  muda por card próprio. Compilar nunca acontece com privilégio.

## Fora de escopo

- **Notificação de bateria baixa.** O limiar, o rearme e o canal de notificação
  são card próprio, aberto junto com este. Ela exige canal novo e dependência
  nova, e é o único item do card original que não se verifica nem com
  `cargo test` nem com olho no painel sem esperar a bateria cair de verdade.
- Modo diagnóstico dentro do binário do applet. Vira exemplo no `mchose-device`.
- EQ, surround e qualquer coisa de áudio — card #4.
- Controlar volume, mudo ou teclas de mídia. Já funcionam como HID padrão.
- Instalar a regra udev ou o desktop entry numa máquina. O card entrega os
  arquivos e o comando; instalar é operação de máquina.
- Empacotamento (`.deb`, COPR, Flatpak).
- Tradução. As frases nascem em português, sem infraestrutura de i18n.
- Ícone próprio desenhado. Usa nome do tema.
- Configuração pelo `cosmic-settings`.

## Arquivos no escopo

- `Cargo.toml`
- `Cargo.lock`
- `cosmic-applet-mchose/**`
- `crates/mchose-device/**`
- `crates/mchose-protocol/**`
- `install/**`
- `AGENTS.md`
- `README.md`
- `docs/specs/3-cosmic-applet/**`

O `crates/mchose-device/src/lib.rs` entra porque hoje reexporta **só**
`DeviceEvent`: `BatteryReading`, `ChargeState` e `FirmwareVersion` ficam
inalcançáveis, e o applet não conseguiria nomeá-los em teste sem depender do
`mchose-protocol` direto — contra a invariante de fronteira do próprio pack.
Entram junto o `crates/mchose-device/AGENTS.md`, cujo blast radius ainda diz
"consumidores diretos: nenhum ainda", e o exemplo `eventos`.

O `crates/mchose-protocol/**` entrou **depois** de escrito este spec, e a
expansão é declarada em vez de silenciosa: `BatteryReading` e `FirmwareVersion`
são `#[non_exhaustive]`, o que impede literal de struct fora do crate que os
define. Reexportá-los não bastava — o applet não conseguia construí-los nem em
teste. Ganharam construtores `new`, que é o padrão para esse atributo. A revisão
do spec previu a metade do problema (o applet não conseguiria nomeá-los); a outra
metade só apareceu no compilador.

## Critério de verificação

```bash
cargo build -p cosmic-applet-mchose --locked
cargo test -p cosmic-applet-mchose --locked
cargo clippy -p cosmic-applet-mchose --locked -- -D warnings \
  -D clippy::indexing_slicing -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic
cargo fmt --check
```

O `--locked` não é zelo: sem ele um build resolve versões novas e executa
`build.rs` novo na máquina de quem revisar.

Testável sem painel e sem hardware, porque é lógica pura de estado:

- cada `DeviceEvent` produz a frase que a tabela acima declara
- `Disconnected` não produz a frase de `NoDevice`, e vice-versa
- `PermissionDenied` produz a frase que menciona a regra udev
- o último percentual sobrevive a `NoResponse` e a `Disconnected`
- `Connected` com `dongle: None` não apaga uma versão já conhecida
- variante desconhecida (o braço `_`) preserva o último percentual e não muda a
  frase
- a saída do exemplo para `Rejected` não contém nenhum byte da entrada
- a frase de `PermissionDenied` não contém `/dev/hidraw`

Verificação que **exige olho humano** e fica declarada como não coberta:

- o applet aparece no painel depois de instalado o desktop entry e acrescentado
  o ID em `plugins_wings`
- o percentual mostrado bate com o que o fone reporta
- o popover abre e mostra as versões de firmware
- a assinatura sobrevive ao dongle sumir e voltar com o painel rodando

Provar a cadeia sem painel é `cargo run -p mchose-device --example eventos`, um
exemplo que imprime o `Stream`. **O binário do applet tem um modo só: o do
painel.** Um ramo de modo no `main` criaria dúvida sobre qual metade do binário
está sendo exercitada.
