# mchose-protocol — Module Pack

Converte bytes HID do headset MCHOSE V9 PRO em tipos, e tipos em bytes.

## 1. Fronteira

**É dono de:** o formato dos pacotes do canal vendor — quais bytes montam uma
consulta, o que os bytes de uma resposta significam, e a regra que decide se um
dispositivo é o V9 PRO.

**Não é dono de, apesar de parecer:**

- Abrir `/dev/hidraw`, enumerar dispositivos, escrever, ler, esperar. Este crate
  nunca viu um descritor de arquivo — é `mchose-device` (card #2) quem fala com
  o kernel.
- **Esperar os 300 ms** entre o `SET_FEATURE` e o `GET_FEATURE`, e **aplicar o
  deadline de 2 s**. Os dois valores moram aqui porque são dados do protocolo
  (`src/request.rs:30` e `:34`), mas quem dorme e quem desiste é o card #2.
- Decidir o que mostrar quando não há leitura. "Dormindo com o último valor
  conhecido" é política de UI, do card #3.
- Registrar em log. O crate expõe o byte estranho; quem escreve linha de log é
  o chamador.

## 2. Regras de negócio e invariantes

- **VID/PID não identifica o produto.** `291d:385d` é compartilhado por S9 PRO,
  G9 PRO, V9 e V9 PRO — o chip C-Media reporta os mesmos ids para produtos
  diferentes. O reconhecimento exige nome junto — `src/device.rs:30`
- **A exclusão de `V9 PRO 2` é avaliada antes da inclusão.** `"MCHOSE V9 PRO 2"`
  contém `"V9 PRO"`; na ordem inversa a exclusão seria decorativa —
  `src/device.rs:45`
- **O nome é entrada controlada pelo dispositivo.** Vem do descritor USB, entra
  como bytes crus, e o que não for UTF-8 válido não reconhece em vez de entrar
  em pânico — `src/device.rs:37`
- **O nome termina no primeiro NUL.** O ioctl devolve buffer de tamanho fixo, e
  o que vem depois do terminador é resto de buffer reaproveitado — pode nem ser
  UTF-8. Validar o buffer inteiro faria um V9 PRO legítimo não reconhecer, em
  silêncio. Achado da revisão do diff, não do spec — `src/device.rs:37`
- **O kernel expõe duas formas do nome para o mesmo dispositivo.**
  `HIDIOCGRAWNAME` devolve `"C-Media Electronics Inc MCHOSE V9 PRO"`; o
  `ATTRS{product}` do udev devolve `"MCHOSE V9 PRO"`. Por isso o casamento é por
  conteúdo e nunca por igualdade — `src/device.rs:45`
- **Todo buffer trocado com o dispositivo tem 64 bytes:** 1 de report id mais os
  63 do descritor. Comprimento é contrato, não detalhe — `src/request.rs:26`
- **A API de request é fechada.** Dois construtores, nenhum que aceite report id
  ou payload do chamador — `src/request.rs:58` e `:67`
- **No request de firmware, dongle é `1` e fone é `0`.** Inverter produz dois
  valores plausíveis que ninguém detecta a olho — `src/request.rs:51`
- **Percentual fora de `0..=100` não é leitura.** Número errado na tela é pior
  que ausência de número — `src/battery.rs:71`
- **Byte de estado desconhecido não invalida a leitura.** O percentual continua
  valendo e o byte é preservado — `src/battery.rs:29`
- **Rejeição só carrega os bytes quando o pacote era de um canal nosso.**
  Tráfego alheio vira `None` — `src/lib.rs:51`
- **`NoReading<'a>` empresta o buffer que você passou.** Nada que o carregue
  dentro pode ser `'static`. Quem precisa atravessar uma fronteira `'static` —
  uma `Message` de UI, um canal entre threads — copia os bytes antes. Foi a
  primeira coisa que o `mchose-device` descobriu ao consumir este crate, e não
  estava escrita em lugar nenhum — `src/lib.rs:33`
- **A forma textual da versão é só exibição.** Comparação de ordem usa os quatro
  bytes crus — `src/firmware.rs:22`
- **Nenhuma entrada causa pânico**, em nenhum tamanho. Acesso sempre por `get`,
  nunca por indexação — `src/lib.rs:10` e os lints do mesmo bloco

## 3. Padrão canônico

**Arquivo de referência: `src/battery.rs`.** Todo decodificador novo copia a
estrutura dele:

1. `let ours = buf.first() == Some(&REPORT_X);` — decide se o pacote é nosso
2. `let reject = || reject(ours, buf, N);` — uma única forma de falhar, onde `N`
   é quantos bytes daquele canal têm campo identificado. O helper mora em
   `src/lib.rs:51` e é compartilhado: registrar byte de significado desconhecido
   num repositório público é o que ele existe para impedir
3. `buf.get(n)` com `let ... else` para cada campo, nunca `buf[n]`
4. validação de faixa por último, com o mesmo `reject()`

Módulo por assunto (`device`, `request`, `battery`, `firmware`), testes no fim do
próprio arquivo em `mod tests`, fixtures sendo **bytes capturados do hardware**
com a data no comentário — nunca bytes inventados.

Documentação e comentário em português; identificadores em inglês, como o resto
do Rust ao redor. Comentário explica *por quê*, não *o quê*.

## 4. Blast radius

**Consumidores diretos:** `mchose-device`, que usa praticamente tudo —
`is_supported` na descoberta (`crates/mchose-device/src/discovery.rs`),
`battery_request`/`firmware_request` e as constantes de tempo no transporte e na
sessão, e `decode_battery`/`decode_firmware` com o `NoReading` na máquina de
estados (`crates/mchose-device/src/machine.rs`). O previsto é o applet do
card #3, que consome `BatteryReading`, `ChargeState` e `FirmwareVersion`
indiretamente, dentro do `DeviceEvent`.

**O que a mudança de um tipo público quebra:** `ChargeState`, `BatteryReading`,
`FirmwareVersion` e `NoReading` atravessam o `mchose-device` e chegam ao applet.
Os três primeiros são `#[non_exhaustive]`, então variante nova não quebra
ninguém; campo novo em `NoReading`, sim.

**Contratos que atravessam processo:** nenhum. Não há rede, IPC, banco ou fila.
O único dado que atravessa fronteira são os bytes do `/dev/hidraw`, e este crate
está de um lado só dela.

**Pontos de registro:** `src/lib.rs` — módulo novo exige `pub mod` lá. É o único.
`Cargo.toml` da raiz lista os membros do workspace, e ganha uma linha quando
`mchose-device` nascer.

**Onde mora a condicional:** só uma, `FirmwareTarget` em `src/request.rs:51`,
que escolhe entre dongle e fone. Não há flag, permissão nem variação por cliente.

**O que muda junto:** `spikes/battery_probe.py` fala o mesmo protocolo. Ele não é
oráculo de comportamento — diverge de propósito em percentual fora de faixa e em
identificação — mas se um byte do protocolo mudar, os dois mudam.

## 5. Armadilhas

**`ChargeState::Charging` quase nunca aparece, e isso não é bug.** O fone se
desconecta do dongle 2.4GHz quando é plugado para carregar. O estado existe no
protocolo, o firmware o define, e na prática o dispositivo some em vez de
reportá-lo. Quem for "consertar" a ausência vai perseguir fantasma.

**`ChargeState::Asleep` (26) nunca foi capturado no hardware.** O mapeamento veio
do código do fabricante. O percentual que acompanha esse estado pode não ser
confiável — o driver oficial prefere o último valor conhecido quando o fone
dorme. O crate devolve o que veio, sem julgar; a decisão é do applet.

**O report id ocupa o byte 0, aqui.** O protocolo foi extraído de código WebHID,
onde `sendReport` recebe o payload **sem** o id e `receiveFeatureReport` devolve
**com** ele. Essa assimetria já causou um erro de índice durante a investigação.
Em `hidraw` o id está sempre no byte 0, nas duas direções.

**Os lints de `clippy` não valem para os testes, de propósito.** O comando de
verificação do spec não usa `--all-targets`: `expect` num teste é como o teste
declara falha, e proibi-lo tornaria teste impossível de escrever. Os lints
existem para o código que recebe bytes do dispositivo.

**`NoReading.rejected` não devolve o buffer inteiro, de propósito.** São só os
bytes com campo identificado — 4 no canal de bateria, 6 no de firmware. O resto
do buffer de 64 B nunca foi mapeado, e quem registra em log costuma colar a linha
num issue público. "Consertar" isso devolvendo tudo reabre o problema.

**Os report ids `0xED` e `0x41` existem no descritor e não têm uso conhecido.**
Não há construtor que os alcance, e isso é deliberado: escrita especulativa em
canal vendor de firmware é caminho conhecido para brickar dispositivo.
