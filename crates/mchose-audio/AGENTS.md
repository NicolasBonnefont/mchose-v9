# mchose-audio — Module Pack

EQ de 10 bandas para o MCHOSE V9 PRO, pela `filter-chain` do PipeWire.

## 1. Fronteira

**É dono de:** o texto da filter-chain, a escrita dele no lugar certo, e a
aplicação de ganho no nó do PipeWire.

**Não é dono de, apesar de parecer:**

- Falar com o dispositivo. **O EQ não é comando do fone.** No Windows é um APO,
  DSP no host; aqui é PipeWire. O crate não conhece HID nem `mchose-device`.
- Reiniciar o `filter-chain.service`. Escrever o arquivo **não** cria o sink;
  quem reinicia é o `install/install-eq.sh`, no padrão da regra udev e do applet.
- Trocar o sink padrão do sistema. Efeito colateral na sessão; quem decide é o
  usuário.
- Surround. Depende de HRIR que o projeto não tem.

## 2. Regras de negócio e invariantes

- **O gerador é puro** e recebe duas coisas: os ganhos e o `node.name` do alvo
  — `src/config.rs:63`.
- **Sem `target.object` o EQ pega o sink errado.** O `sink-eq6.conf` do sistema
  não tem esse campo, e por isso segue o sink *padrão*, não o fone.
- **O nome do alvo é recusado, nunca sanitizado** — `src/config.rs:55`. Ele vem
  das strings USB que o próprio dispositivo declara (`manufacturer`, `product`,
  `serial`), e um `x" } plugin = "/tmp/e.so` fecharia o bloco SPA-JSON e faria o
  PipeWire carregar um `.so` no processo da sessão, no login.
- **Ganho fora de `-12..=+12`, NaN e infinito são recusados.**
- **O nome do arquivo é constante** e o destino é o diretório informado, nunca
  derivado de entrada: `~/.config/` é diretório de execução, e nome derivado
  permitiria travessia para `autostart/` — `src/install.rs:41`.
- **Arquivo sem o marcador do crate não é sobrescrito** — `src/install.rs:50`.
  `effect_input.eq6` e vizinhos são do usuário.
- **A escrita é atômica** (tmp + rename), e entrada inválida não deixa rastro:
  a geração acontece antes de qualquer toque em disco — `src/install.rs:57`.
- **A identificação do sink é delegada ao `mchose-protocol`** — `src/pipewire.rs:62`.
  Casar só por VID/PID pegaria S9 PRO, G9 PRO e V9; prefixo de nome pegaria o
  `V9 PRO 2`.
- **Só `media.class = Audio/Sink` conta** — `src/pipewire.rs:71`. O microfone do
  mesmo fone tem `alsa.components` e `alsa.card_name` idênticos.
- **Nada de shell e nada de FFI** — `src/pipewire.rs:147`. `argv` separado, e o
  nó endereçado por id numérico. O `unsafe` do projeto segue confinado ao
  `mchose-device`.
- **O resultado vem da releitura, não do estado do nó** — `src/pipewire.rs:189`.

## 3. Padrão canônico

**Arquivo de referência: `src/config.rs`.** Núcleo puro, testável sem PipeWire —
o mesmo desenho do `mchose-protocol`. `src/pipewire.rs` e `src/install.rs` são a
camada fina de I/O em volta dele.

Em `src/pipewire.rs`, ao percorrer o dump, use `else { continue }` e **nunca**
`?`: o `?` num laço aborta a função inteira no primeiro objeto sem o campo — e
o dump real começa com Core, Client e Device, todos sem `info.props`. Foi
exatamente assim que a primeira versão não achava o fone com o dongle plugado.

## 4. Blast radius

**Consumidores diretos:** `examples/eq.rs`, que é o consumidor real. Um futuro
card de UI consome `achar_no_do_eq` para valor inicial de slider — por isso o
`node.name` do sink virtual é estável e público.

**Do que depende:** `mchose-protocol` (só `is_supported`) e `serde_json`.

**Contratos que atravessam processo:** `pw-dump` e `pw-cli` como binários, e o
arquivo em `filter-chain.conf.d/` que o PipeWire lê no login.

**Pontos de registro:** `src/lib.rs` para módulo novo; `install/install-eq.sh`
para a instalação; `Cargo.toml` da raiz para o membro.

## 5. Armadilhas

**Instalar não é aplicar.** Escrever o arquivo não cria o sink — ele só nasce
depois de `systemctl --user restart filter-chain.service`. E texto recusado vira
**aviso no log**, não erro: os outros fragmentos carregam, o nosso some, e
ninguém é avisado. Sempre conferir o nó depois.

**"Suspenso implica não aplicado" é falso como regra.** Foi observado numa
instância própria de PipeWire (`pipewire -c`) e **não** se reproduz sob o
`filter-chain.service`. Não deduza do estado; releia.

**A faixa de ganho e as frequências são escolha nossa** — `src/config.rs:14`.
Não há dado do fabricante para este fone: a tabela `EQ_HID_CMD` do bundle é da
família `thx`.
