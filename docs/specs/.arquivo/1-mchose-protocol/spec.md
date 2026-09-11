# 1 — mchose-protocol: pacotes HID de bateria e firmware

**Card:** https://github.com/NicolasBonnefont/mchose-v9/issues/1
**Módulo:** `crates/mchose-protocol`
**Status:** rascunho

## Comportamento atual

Não existe código. `crates/` não existe.

O único artefato que fala o protocolo é `spikes/battery_probe.py`, validado
contra o hardware. Ele é prova de que os bytes vieram do dispositivo — não
oráculo de comportamento. Diverge do que este card precisa em três pontos:

- `spikes/battery_probe.py:29` identifica o dispositivo **só por VID/PID**.
  Como `291d:385d` é compartilhado por S9 PRO, G9 PRO, V9 e V9 PRO, o probe
  pode abrir o fone errado. Funcionou por acidente: só há um desses
  dispositivos nesta máquina.
- `spikes/battery_probe.py:58` aceita o pacote com `len >= 4` e não valida
  faixa nenhuma: devolve `buf[2]` como percentual seja ele qual for. O
  mapeamento de estado com preservação do byte desconhecido o probe já faz
  (`:59`) — o que falta é a validação.
- `spikes/battery_probe.py:54` concatena os bytes de versão sem validar faixa.

## Comportamento alvo

Um crate `mchose-protocol` que converte bytes em tipos e tipos em bytes, sem
tocar em I/O.

**Bateria.** Produz o request como report `0x55` com payload
`[0x65, 0x01, 0x00 ×61]`. Interpreta a resposta, que chega como report `0x55`
com prefixo `0x65`, extraindo percentual e estado de carga. O mesmo
decodificador serve a resposta solicitada e ao push espontâneo.

**Estado de carga.** `2` descarregando, `3` carregando, `4` cheia, `26`
dormindo. Qualquer outro valor é preservado como estado desconhecido, com o
byte original acessível.

`26` (dormindo) nunca foi capturado no hardware. O crate devolve o percentual
como veio, sem julgar; preferir o último valor conhecido é decisão do applet
(card #3). Quando um pacote `55 65 xx 1a` real for capturado, ele entra nos
testes.

**Firmware.** Produz o request do feature report `0xAA`, onde o byte 2 vale `1`
para o dongle e `0` para o fone. Interpreta a resposta, que começa com
`[170, 1]`, devolvendo os quatro bytes crus de versão (posições 2 a 5). A forma
textual que o fabricante compara — cada byte como um dígito decimal,
`00 00 01 02` → `"0012"` — é derivada sob demanda, não armazenada, porque a
concatenação perde informação se algum byte passar de 9.

**Identificação do dispositivo.** Reconhece quando o VID/PID é `291d:385d`
**e** o nome do produto contém `V9 PRO` **e** não contém `V9 PRO 2`. A exclusão
é avaliada antes do casamento positivo — sem isso ela é decorativa, porque
`"MCHOSE V9 PRO 2"` contém `"V9 PRO"`. Casamento por conteúdo, nunca por
igualdade exata, porque o kernel expõe **duas formas** do nome (ver
Invariantes). Só o V9 PRO é reconhecido: decisão da entrevista deste card,
contra a alternativa de já nascer com tabela para os irmãos, que não temos como
validar.

**Postura diante do inesperado: tolerante.** Decisão da entrevista. Percentual
fora de `0..=100` invalida a leitura, porque número errado na tela é pior que
ausência de número — mas **sem descartar o pacote**: o erro carrega os bytes
crus, senão o caso que mais interessa mapear é justamente o único que o
consumidor não consegue registrar. Estado desconhecido **não** invalida: o
percentual continua valendo e o applet mostra o número omitindo o estado.

São exatamente dois os bytes inesperados que o crate preserva: o estado de
carga fora de `{2, 3, 4, 26}` e o dígito de firmware maior que 9. Ambos ficam
acessíveis como valor tipado, não como despejo de buffer: o crate não oferece
API que devolva o pacote inteiro para logging, não tem canal de diagnóstico
próprio e não depende de biblioteca de log. Quem registra é o chamador.

**Tráfego alheio é descartado em silêncio.** O mesmo `/dev/hidraw` carrega
teclas de mídia (Consumer, report `0x06`) e telefonia (report `0x07`), que
chegam a cada toque de volume ou mute em uso normal. Report que não pertence ao
canal vendor — `0x06`, `0x07`, `0xED`, `0x41` — não é anomalia e não vira
registro. Só vira registro o `0x55` ou `0xAA` com conteúdo inesperado.

**Prefixo que não casa, pacote curto demais e percentual fora de faixa são a
mesma coisa para quem consome:** não há leitura. O crate devolve ausência, não
uma taxonomia de erros.

**Constantes de tempo.** O crate é a fonte única dos dois tempos do protocolo,
expostos como constantes nomeadas:

| Constante | Valor | Procedência |
| --- | --- | --- |
| intervalo entre `SET_FEATURE` e `GET_FEATURE` do `0xAA` | **300 ms** | `spikes/battery_probe.py:45`, validado no hardware |
| deadline de resposta de bateria | **2 s** | default do fabricante: `sendReportOnceSync` declara `timeOut: s = 2e3` |

O deadline resolve uma divergência que existia entre os artefatos: o design
dizia 2 s e o probe usava 3 s, número sem origem, escolhido ao escrever o
probe. Fica o valor do fabricante, que tem procedência, e o probe foi alinhado
para não haver duas verdades no repositório.

São dados do protocolo, não política do consumidor. O crate **não** sequencia
nada: quem espera os 300 ms e quem aplica o deadline é o card #2.

## Invariantes

- **O crate não faz I/O.** Não abre arquivo, não chama syscall, não depende de
  runtime assíncrono. É o que permite testá-lo sem hardware e extraí-lo inteiro
  depois, seja para um daemon, seja para um PR ao HeadsetControl
  (`AGENTS.md`, seção Arquitetura).
- **O crate é `#![forbid(unsafe_code)]` e sem dependências fora da `std`.** Um
  decodificador de bytes puro não precisa de `unsafe` nem de terceiros; recusar
  ambos elimina superfície de cadeia de suprimentos num crate cuja única
  entrada é hardware não confiável.
- **Nenhuma entrada de qualquer tamanho causa pânico.** Todo acesso a byte usa
  acesso verificado (`get`/`try_into`), nunca indexação direta. Buffer vazio,
  truncado no meio do prefixo, ou maior que 64 B retorna ausência de leitura —
  nunca `panic!`, `unwrap` ou `expect`. Vale igualmente para bateria e
  firmware; o oráculo só é seguro porque opera sobre buffer de tamanho fixo, e
  o crate **não** herda essa premissa.
- **A identificação casa as duas formas do nome que o kernel expõe.**
  `HIDIOCGRAWNAME` devolve `C-Media Electronics Inc MCHOSE V9 PRO`; o
  `ATTRS{product}` do udev, no mesmo dispositivo, devolve `MCHOSE V9 PRO`. O
  card #2 enumera por ioctl e reata por monitor udev no hotplug — comparar por
  igualdade faria o applet nunca reencontrar o fone depois de um replug, sem
  erro nenhum. Casamento por conteúdo, e nunca só por VID/PID
  (`AGENTS.md`, seção Fronteiras).
- **A API de request é fechada, não genérica.** Exatamente dois construtores —
  bateria e firmware (dongle ou fone) — e nenhuma função que aceite report ID
  ou payload arbitrário do chamador. Os report IDs `0xED` e `0x41` não são
  alcançáveis por nenhum caminho público. Escrita especulativa em canal vendor
  de firmware é caminho conhecido para brickar dispositivo.
- **Todo buffer de request tem exatamente 64 bytes:** 1 de report ID mais os 63
  de payload que o descritor declara para `0x55` (Input+Output) e `0xAA`
  (Feature). Comprimento é parte do contrato, não detalhe do consumidor.
- **O report ID ocupa o byte 0 do buffer, na escrita e na leitura.** É a
  convenção do `hidraw` e difere da do WebHID de onde o protocolo foi extraído
  — ver `README.md`, seção "O protocolo, e como ele foi obtido".
- **A forma textual da versão é só exibição.** Nenhuma comparação de ordem é
  feita sobre ela: quem comparar versões compara os quatro bytes crus, porque a
  concatenação do fabricante deixa de ser ordenável assim que um byte passa
  de 9.
- **A filtragem por nome é correção funcional, não controle de acesso.**
  `install/99-mchose-v9.rules` casa apenas VID/PID e concede, via `uaccess`,
  leitura e escrita no `hidraw` a todo processo do usuário da sessão gráfica.
  Casar o nome impede o applet de falar com o fone errado; não impede outro
  processo de falar com este fone. O crate não tem, e não deve aparentar ter,
  papel de fronteira de privilégio.
- **Os tipos públicos nascem `#[non_exhaustive]`.** O protocolo continua sendo
  mapeado depois deste card; estado novo não pode quebrar #2 e #3 de surpresa.
- **Fixture é byte de protocolo, não captura de sessão.** Os bytes versionados
  nos testes são os já publicados neste spec. Nenhum dump bruto novo de
  `hidraw` entra no repositório sem que cada campo tenha sido identificado,
  porque o repositório é público e o conteúdo de `0xED` e `0x41` é desconhecido.
- **Os bytes capturados no hardware continuam decodificando igual.**
  `55 65 46 02` → 70%, descarregando. `aa 01 00 00 01 02 ff 25` → `"0012"`.

## Fora de escopo

- Abrir `/dev/hidraw`, enumerar, hotplug, loop de leitura — card #2. Este crate
  não sabe que `hidraw` existe.
- Sequenciar a troca. O crate declara as constantes de tempo, mas quem espera
  os 300 ms e quem aplica o deadline é o #2.
- EQ, surround e qualquer coisa de áudio — card #4.
- Uso dos report IDs `0xED` e `0x41`. Existem no descritor, não têm uso
  conhecido, e inventar um seria especular.
- Tabela de dispositivos irmãos. Decidido na entrevista: só V9 PRO.
- Escrita de **configuração** no dispositivo. O crate emite escritas no canal
  vendor (output `0x55` e feature-set `0xAA`), mas apenas os dois requests de
  consulta; nenhum pacote que altere estado persistente do fone.
- OTA de firmware.
- Scaffold dos outros crates. O `Cargo.toml` da raiz declara **um** membro;
  `mchose-device`, `mchose-audio` e o applet nascem nos cards deles.
- Dependências externas. Quatro tipos de pacote, aritmética de byte e
  comparação de string não precisam de `serde`, `thiserror` nem `tracing`.
- CI. O remote existe e este é o único crate que roda sem hardware, mas o
  workflow fica para um card próprio; os comandos rodam à mão.

## Arquivos no escopo

- `Cargo.toml` (raiz) — workspace com `resolver = "2"`; `members` lista só
  `crates/mchose-protocol`
- `crates/mchose-protocol/**`, incluindo `crates/mchose-protocol/AGENTS.md`,
  que é o Module Pack que #2, #3 e #4 vão ler antes de consumir o crate
- `rust-toolchain.toml` — a toolchain fica fixada aqui
- `Cargo.lock` — passa a ser versionado
- `AGENTS.md` (raiz) — *Stack real*, *Comandos* e *Rede de segurança
  automatizada* deixam de ser verdade com este card

## Critério de verificação

```bash
cargo test -p mchose-protocol
cargo clippy -p mchose-protocol -- -D warnings \
  -D clippy::indexing_slicing -D clippy::unwrap_used \
  -D clippy::expect_used -D clippy::panic
cargo fmt --check
```

Decodificação, com os bytes reais capturados do hardware:

- `55 65 46 02 00 00 00 00` → 70%, descarregando
- `aa 01 00 00 01 02 ff 25` → versão `"0012"`
- `aa 01 00 00 03 06 ff 25` → versão `"0036"`

Construção:

- o request de bateria tem `len() == 64` e começa com `[0x55, 0x65, 0x01]`
- o request de firmware tem `len() == 64`; dongle é `[0xAA, 0x01, 0x01]` e fone
  é `[0xAA, 0x01, 0x00]`, ambos seguidos de zeros

Identificação:

- `"C-Media Electronics Inc MCHOSE V9 PRO"` (forma do ioctl) é reconhecido
- `"MCHOSE V9 PRO"` (forma do udev) é reconhecido
- `"MCHOSE V9 PRO 2"` **não** é reconhecido, apesar de conter `V9 PRO`
- `"MCHOSE V9 PRO 2 ULTRA"` **não** é reconhecido
- VID/PID correto com nome de outro produto não é reconhecido
- nome vazio e nome com 4096 bytes arbitrários não são reconhecidos e não
  causam pânico

Robustez:

- bateria: buffers de 0, 1, 2 e 3 bytes devolvem ausência, sem pânico
- firmware: buffers de 0 a 5 bytes devolvem ausência, sem pânico (o campo de
  versão exige índice 5)
- buffer de 128 bytes com prefixo válido decodifica, ignorando o excedente
- buffer sem o report ID (`65 46 02 00 ...`) é **rejeitado**, não decodificado
  como 2%; idem `01 00 00 01 02 ...` no firmware
- input report começando com `0x06` (tecla de mídia, mesmo descritor) é
  descartado sem erro e sem registro
- status desconhecido preserva o percentual e expõe o byte
- percentual 200 invalida a leitura e o erro devolve os bytes originais

`spikes/battery_probe.py` continua no repositório como prova de que os bytes
vieram do hardware, não como oráculo de comportamento: a paridade vale **apenas
para os pacotes capturados**. Fora deles a divergência é intencional e está
listada em *Comportamento atual*.
