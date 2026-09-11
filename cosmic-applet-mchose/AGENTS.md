# cosmic-applet-mchose — Module Pack

Mostra a bateria do MCHOSE V9 PRO no painel do COSMIC.

## 1. Fronteira

**É dono de:** o que o usuário vê — a frase de cada estado, o percentual no
painel, o conteúdo do popover, e quando pedir uma leitura nova.

**Não é dono de, apesar de parecer:**

- Falar com o dispositivo. Ele consome `DeviceEvent` do `mchose-device` e não
  conhece `hidraw`, ioctl nem protocolo.
- Decidir o que a UI mostra. Isso é `src/state.rs`, que **não conhece
  libcosmic**; `src/app.rs` só traduz evento em `Message` e estado em widget.
- Reconectar. Quem reata o dispositivo é o `mchose-device`; o applet só reabre o
  fluxo quando ele acaba de vez.

## 2. Regras de negócio e invariantes

- **O estado vive fora do libcosmic.** `src/state.rs:34` recebe `DeviceEvent` e
  devolve frase, percentual e firmware. É essa fronteira que torna as sete
  frases testáveis sem painel — sem ela nada seria verificável.
- **`Disconnected` não diz "carregando".** `Disconnected` é o dongle saindo do
  USB, sempre transitório. O fone plugado para carregar deixa o dongle vivo e o
  rádio mudo, o que é `NoResponse` — e é lá que a frase de carregar mora
  (`src/state.rs:53`).
- **`PermissionDenied` nomeia a regra e não mostra o caminho do device**
  (`src/state.rs:63`). O repositório é público e log colado vai parar lá.
- **`Rejected` não muda a UI e seus bytes não chegam a lugar nenhum**
  (`src/state.rs:69`).
- **Em `ChargeState::Asleep` o percentual não é aplicado.** O pack do protocolo
  registra que esse estado nunca foi capturado no hardware, então o valor que
  vem com ele é duvidoso: mantém-se o último bom e muda-se só a frase
  (`src/state.rs:34`).
- **O último percentual sobrevive a silêncio e a remoção.** Some o estado, não o
  número — apagar faria o painel piscar a cada oscilação do rádio.
- **Versão de firmware conhecida não é apagada por um `Connected` com `None`**
  (`src/state.rs:34`). `None` é "não respondeu agora", não "deixou de existir".
- **O braço `_` é obrigatório e não decide nada** (`src/state.rs:73`):
  `DeviceEvent` é `#[non_exhaustive]`. Inalcançável a partir daqui, então sem
  teste possível — a tabela de estados do spec é o único lugar que cobra.
- **O identificador da assinatura é constante** (`src/app.rs:71`). Derivá-lo de
  estado recriaria a thread, o fd `O_RDWR` e o socket do monitor a cada mudança.
- **Fluxo que acaba vira estado visível e nova tentativa**, nunca fim mudo
  (`src/app.rs:21`, a espera entre tentativas).
- **O applet não expõe socket, porta nem serviço que aceite comando.** Ele é o
  único processo da sessão com o fd do canal vendor aberto; qualquer IPC o
  transformaria em proxy de escrita para o firmware.

## 3. Padrão canônico

**Arquivo de referência: `src/state.rs`.** Comportamento novo entra ali, com
teste, e só então ganha widget em `src/app.rs`. O caminho inverso — lógica dentro
do `update()` — é o que a fronteira existe para impedir.

**A API do libcosmic se lê na fonte, não no doc.** O checkout do rev fixado tem
`examples/applet/src/window.rs`, que é o template canônico desta versão. O doc
oficial manda declarar `cosmic = { version = "1.0" }` como se fosse crate
publicado; o `cosmic` do crates.io é um build tool de C/C++ sem relação, parado
na `0.1.0`.

Documentação e comentário em português; identificadores em inglês.

## 4. Blast radius

**Consumidores diretos:** nenhum. É o topo da pilha.

**Do que depende:** `mchose-device` (todo o `DeviceEvent`, a `Demand` e
`events()`) e o libcosmic no rev `d4d71fd5`.

**Contratos que atravessam processo:** o painel lança o binário pelo `Exec=` do
desktop entry, e lê a lista de applets de
`~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_wings`.

**Pontos de registro:** `install/com.github.NicolasBonnefont.mchose-v9.Applet.desktop`
(o `Exec=` é caminho absoluto, de propósito) e o `plugins_wings`, que o
`install/install-applet.sh` edita. Esquecer o segundo instala o applet sem ele
aparecer.

## 5. Armadilhas

**A árvore só é reprodutível com `--locked`.** Sete pacotes vêm de branch móvel
do `pop-os` sem `rev` no manifesto — entre eles o `cosmic-config-derive`, que é
proc-macro e **executa em tempo de compilação**. O `rev` do libcosmic fixa uma
fonte; as outras sete só o `Cargo.lock` segura. Comando sem `--locked` em
documentação pública é como código novo entra na máquina de quem revisa.

**`Demand::refresh()` sem sessão enfileira, não vira no-op.** O canal tem
capacidade 4: abrir o popover quatro vezes com o dongle fora produz uma rajada de
consultas quando ele voltar. Inofensivo hoje, e registrado para não ser
redescoberto como bug.

**O painel está ancorado embaixo nesta máquina**, não em cima — `anchor: Bottom`
na configuração. Quem for capturar tela para verificar precisa recortar a faixa
inferior.

**`grim` não captura no COSMIC:** o compositor não implementa
`wlr-screencopy-unstable-v1`. A ferramenta que funciona é `cosmic-screenshot
--interactive=false --save-dir`.

**`desktop-file-validate` reprova `Categories=COSMIC;`** por não ser categoria
registrada no freedesktop. É o que os applets oficiais usam; consistência com a
plataforma vence a validação. Não "corrigir" para `X-COSMIC`.

**`pkill -f cosmic-panel` mata o próprio shell** que rodou o comando, porque a
linha de comando dele contém o padrão. Usar `pkill -x cosmic-panel`.

**O binário de debug passa de 400 MB e demora a subir.** Para uso real, compilar
em release.
