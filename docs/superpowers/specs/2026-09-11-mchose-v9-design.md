# mchose-v9 — bateria e controles do fone MCHOSE V9 PRO no COSMIC

Data: 2026-09-11
Máquina: Pop!_OS, COSMIC 1.0.0 (`cosmic-comp` a830784), kernel 7.1.5-76070105-generic

## Problema

O headset MCHOSE V9 PRO liga por dongle 2.4GHz e não expõe bateria de nenhuma
forma que o Linux entenda. Ele não implementa a *HID Battery Strength* padrão,
então não nasce entrada em `/sys/class/power_supply/` e o UPower não publica
nada. No Windows existe o **MCHOSE HUB**, que mostra bateria, EQ e mais; no
Linux não há absolutamente nada.

Registrar a bateria via `uhid` como `power_supply` **não resolveria**: o applet
de bateria do COSMIC assina apenas o DisplayDevice do UPower
(`device_subscription(0)` em `cosmic-applet-battery/src/app.rs`) e mantém um
`battery_percent` único, não uma coleção — ele ignora periféricos por
construção. Para o número aparecer no painel é preciso applet próprio.

Objetivo: um applet COSMIC que mostre bateria e estado do fone, e ofereça os
controles que o M HUB oferece.

## Hardware — o que foi apurado

USB `291d:385d` (dec `10525`/`14429`), "C-Media Electronics Inc MCHOSE V9 PRO".
O dongle enumera quatro interfaces:

| Interface | Classe | Papel |
| --- | --- | --- |
| `3-6:1.0` | `03` HID | canal vendor — é o que nos interessa |
| `3-6:1.1` a `1.3` | `01` Audio | placa de som (`card 1: MCHOSE V9 PRO`) |

O descritor de report da interface HID declara quatro coleções vendor:

| Report ID | Usage Page | Tipo | Tamanho |
| --- | --- | --- | --- |
| `0xED` | `0xFF21` | Input + Output + Feature | 1 B / 1 B / 15 B |
| `0xAA` | `0xFF22` | Feature | 63 B |
| `0x55` | `0xFF90` | Input + Output | 63 B |
| `0x41` | `0xFF82` | Input + Output | 63 B |

O resto do descritor é teclas de mídia (Consumer) e controle de telefonia
(atender/mudo), que já funcionam como HID padrão e não precisam de nós.

**O VID/PID não identifica o produto.** O mesmo `10525:14429` é usado por
S9 PRO, G9 PRO, V9 e V9 PRO — o chip C-Media reporta IDs idênticos para
produtos diferentes. O próprio código da MCHOSE desambigua pela string de nome,
e chega a anexar `" PID14429"` ao nome quando o par bate. Qualquer casamento
nosso tem que usar VID/PID **e** nome.

## Protocolo — o que foi apurado

### Origem

O driver Windows (`~/Downloads/MCHOSE HUB/`) é Electron. O main process está
compilado em bytecode V8 (`out/main/index.jsc`), mas ele carrega a UI de
`https://www.mchose.com.cn/`, e é no bundle web que está a lógica de
dispositivo. A implementação fica no chunk `purify.es-*.js`, função `C7t`
(exportada como `getHeadphonesBatteryInfo`).

Cuidado ao rebaixar: esse chunk tem **7,7 MB** e truncou silenciosamente em
1 MB com `curl --max-time 30`, o que levou a uma conclusão errada antes de o
erro ser percebido. Conferir `Content-Length`.

### Bateria

Output report `0x55`, payload de 63 B = `[0x65, 0x01, 0x00 ×61]`.
Resposta: input report `0x55` começando com `0x65`.

Via `hidraw` o report ID entra no buffer:

```
escreve  [0x55, 0x65, 0x01, 0x00...]   (64 B)
lê       [0x55, 0x65, <bateria %>, <status>, ...]
```

Status: `2` descarregando · `3` carregando · `4` cheia · `26` dormindo.

Idêntico ao `55 65 01 00` do PR #563 do HeadsetControl (MCHOSE X9, mergeado em
22/08/2026), apesar de o X9 ser de outra família de firmware — a função de
bateria é comum às duas.

**Captura real (11/09/2026, hardware deste projeto):**

```
-> 55 65 01 00 ...
<- 55 65 46 02 00 00 00 00      # 0x46 = 70%, status 2 = descarregando
```

### Push espontâneo

Existe `listenChargeStatusChange`, que só registra listener de `inputreport`
`0x55` com prefixo `0x65` e **não envia nada**. O fone empurra atualização
sozinho. O modo normal de operação é passivo: sem polling.

### Firmware

Feature report `0xAA`: envia `[0x01, <1=dongle|0=fone>, 0...]`, espera 300 ms,
lê o feature `0xAA`. Resposta válida começa com `[170, 1]`; a versão são os
bytes 2 a 5 concatenados como dígitos decimais.

**Capturas reais:**

```
dongle  aa 01 00 00 01 02 ff 25   -> "0012"
fone    aa 01 00 00 03 06 ff 25   -> "0036"
```

O app do fabricante usa isso como portão: só mostra bateria se a versão do
dongle for `>=` um mínimo remoto (`enableBatteryVersion`). O nosso dongle passa.

### O que NÃO está no firmware

O canal HID tem exatamente **dois** usos no código do fabricante: bateria e
firmware. EQ, surround 7.1 e redução de ruído do microfone não são comandos de
dispositivo — são **APOs do Windows** (`libs/V9_PRO/02_APO/CmApoIce.dll`,
classe `AudioProcessingObject`, registrada como "C-Media Audio Effects" com
Playback EFX/MFX/SFX e Recording EFX/MFX). É DSP rodando no host.

Consequência: no Linux esses recursos não se obtêm falando com o fone, e sim
processando áudio localmente. PipeWire faz isso nativamente.

### Armadilha: "carregando" é praticamente inalcançável

O PR #563 documenta, e a topologia confirma, que o fone **se desconecta do
dongle 2.4GHz quando é plugado para carregar**. O status `3` existe no
protocolo mas quase nunca chega. A UI não pode tratar desaparecimento como
"desligado", sob pena de mentir todo dia.

## Escopo

Nesta versão:

- bateria (%, estado, dormindo) com atualização por push
- versão de firmware do dongle e do fone
- EQ de 10 bandas e surround, via PipeWire
- hotplug: dongle pode sumir e voltar a qualquer momento

Fora desta versão:

- volume dos avisos sonoros e auto-desligamento — no M HUB para dispositivos
  C-Media isso passa pela ConfLib (`PropertyControl` → driver de kernel
  `MCHOSE_V9_PRO.sys`), que não tem equivalente no Linux. **Inalcançável, não
  adiado:** não há máquina Windows disponível (confirmado em 11/09/2026) para
  capturar o tráfego USB. Sobra reversão estática das DLLs e do `.sys` que estão
  em `~/Downloads/MCHOSE HUB/` — caro e sem garantia.
- OTA de firmware. Risco alto, benefício baixo.

## Arquitetura

Binário único: o applet é dono do `hidraw`. Um applet de painel vive enquanto o
painel vive, então um daemon separado adicionaria IPC e ciclo de vida sem
comprar quase nada. O protocolo fica isolado desde o primeiro dia para poder
sair inteiro depois — seja para um daemon, seja para um PR ao HeadsetControl.

```
crates/mchose-protocol/   sem I/O, puro
crates/mchose-device/     hidraw + hotplug por udev
crates/mchose-audio/      EQ e surround via PipeWire
cosmic-applet-mchose/     applet libcosmic
```

### mchose-protocol

Não abre arquivo nenhum: recebe e devolve bytes. Contém a construção de
requests, o parse de respostas e a regra de identificação do dispositivo
(VID/PID **mais** nome, excluindo explicitamente `V9 PRO 2`, que usa outro
caminho no firmware). Por ser puro, é testável inteiramente com os bytes
capturados acima e é a unidade que se extrai para outros destinos.

### mchose-device

Enumera `/dev/hidraw*` via `HIDIOCGRAWINFO`, filtra pelo protocolo, abre e
mantém loop assíncrono de leitura, emitindo eventos. Monitora udev para
hotplug — o dongle desaparecer não pode derrubar o applet, e reconexão é
automática.

### mchose-audio

Gera e controla uma `filter-chain` do PipeWire (sink virtual com EQ e
surround).

> **Decisão pendente de confirmação:** optou-se por uma filter-chain própria em
> vez de integrar com o EasyEffects, para não depender de um app externo estar
> instalado. O custo é que atualização de parâmetro ao vivo é mais trabalhosa.
> Se a preferência for EasyEffects, esta seção muda e o resto da spec não.

### cosmic-applet-mchose

Applet libcosmic: ícone com percentual no painel, popover com detalhes,
firmware e controles de áudio. Notificação de bateria baixa.

## Estados

O hardware mente por omissão, então os estados são parte do contrato:

| Estado | Detecção | UI |
| --- | --- | --- |
| Sem dongle | nenhum `hidraw` casa | ícone apagado |
| Dongle sem fone | sem resposta em 2 s | "fone desligado" |
| Dormindo | status `26` | "dormindo" + último % conhecido |
| Carregando | status `3` | raro; ver armadilha acima |
| Normal | status `2` ou `4` | % + estado |

Desaparecimento súbito deve ser comunicado como "desconectado (pode estar
carregando)", nunca como "desligado".

## Instalação

**Permissões.** `/dev/hidraw*` nasce root-only. `install/99-mchose-v9.rules`
aplica `TAG+="uaccess"`, que entrega o descritor ao usuário da sessão gráfica
via ACL do systemd-logind — sem grupo novo, sem `chmod` no boot. A regra casa
por VID/PID, que é mais largo que o dispositivo; quem filtra pelo nome é o
daemon.

**Applet.** Desktop entry em `/usr/share/applications/` (ou
`~/.local/share/applications/`) com `Type=Application`, `Categories=COSMIC;`,
`NoDisplay=true`, `X-CosmicApplet=true` e `Icon=<id>-symbolic`. O painel lê a
lista de applets ativos de
`~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_wings` (formato
RON). Applets de terceiros funcionam nesta máquina — já há um instalado
(`com.github.hmrdsmoke.soulless-launcher.Applet`).

**Toolchain.** Rust não está instalado. Usar `rustup`; o `rustc` 1.75 do apt é
velho demais para libcosmic.

## Testes

`mchose-protocol` recebe testes de unidade com os bytes reais capturados neste
documento — `55 65 46 02` e `aa 01 00 00 01 02 ff 25` — não com fixtures
inventadas.

`mchose-device` é testado contra um V9 PRO virtual criado com **`uhid`**, que
responde com os bytes gravados. Permite exercitar o que é difícil à mão: dongle
sumindo no meio de uma leitura, fone dormindo, resposta truncada, resposta que
nunca vem.

**Custo não previsto na primeira redação:** `/dev/uhid` é `crw------- root root`
e o módulo não está carregado. Esses testes exigem root ou regra udev própria —
não rodam num CI sem privilégio, ao contrário do que esta seção afirmava antes.
Os testes de `mchose-protocol`, esses sim, rodam em qualquer lugar.

`spikes/battery_probe.py` é o probe em Python que validou o protocolo no
hardware. Fica no repositório como referência executável e oráculo: a
implementação Rust deve produzir os mesmos bytes.

## Riscos

**O EQ é o maior pedaço e o menos explorado.** Bateria está validada ponta a
ponta; a filter-chain do PipeWire ainda não teve nenhum spike. É o item com
maior chance de estourar a estimativa, e por isso é o último na ordem de
construção — nada depende dele.

**Churn de API do libcosmic.** É pré-1.0 em termos de estabilidade de API,
ainda que o COSMIC já esteja em 1.0. Fixar versão e não perseguir `main`.

**Os dois recursos fora de escopo não têm caminho barato.** Sem máquina
Windows, captura USB está descartada; sobra reversão estática de DLL. Se virarem
requisito, o custo é de outra ordem de grandeza — tratar como projeto separado.
