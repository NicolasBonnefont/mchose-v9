# mchose-v9

Bateria e controles do headset **MCHOSE V9 PRO** no Linux, com applet para o
COSMIC.

O fone liga por dongle 2.4GHz e não implementa a *HID Battery Strength* padrão.
Consequência: não nasce entrada em `/sys/class/power_supply/`, o UPower não
publica nada, e nenhum ambiente gráfico tem o que mostrar. No Windows existe o
MCHOSE HUB; no Linux não havia nada.

> [!NOTE]
> Registrar a bateria como `power_supply` **não** resolveria: o applet de bateria
> do COSMIC assina apenas o DisplayDevice do UPower e mantém um valor único, não
> uma coleção — ele ignora periféricos por construção. Para o número aparecer no
> painel é preciso applet próprio.

## Estado

| Parte | Situação |
|---|---|
| Protocolo (bateria, firmware) | pronto, validado no hardware |
| Acesso ao dispositivo, hotplug | pronto, validado no hardware |
| Applet no painel do COSMIC | pronto, funcionando no painel |
| EQ e surround via PipeWire | não começou |

O applet mostra a bateria no painel. O que falta é o EQ.

## Instalação

Ainda não há pacote. Para desenvolver:

```bash
# dependencias de sistema
sudo apt install libudev-dev libpipewire-0.3-dev libxkbcommon-dev \
  libxkbcommon-x11-dev wayland-protocols libinput-dev libvulkan-dev \
  libegl1-mesa-dev libgl1-mesa-dev libfontconfig1-dev libfreetype6-dev \
  libexpat1-dev libgbm-dev libdrm-dev libdbus-1-dev libx11-dev libxcb1-dev \
  libxcursor-dev libxi-dev libxrandr-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# acesso ao dispositivo
sudo cp install/72-mchose-v9.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --action=add --subsystem-match=hidraw

cargo test --locked
cargo build --release --locked -p cosmic-applet-mchose
bash install/install-applet.sh      # instala o applet e o registra no painel
pkill -x cosmic-panel               # o painel reinicia sozinho
```

> [!WARNING]
> **O número do arquivo da regra udev importa.** Quem converte a tag `uaccess` em
> ACL é o `73-seat-late.rules`. Uma regra numerada acima de 73 marca o dispositivo
> depois desse estágio já ter passado: a tag aparece em `CURRENT_TAGS` e nada
> acontece. Foi o que houve enquanto a regra se chamava `99-`.

Sem a regra, `/dev/hidraw*` fica `crw------- root root` e só `sudo` funciona.

## O protocolo, e como ele foi obtido

O driver oficial (`MCHOSE HUB`) é Electron. O processo principal está compilado
em bytecode V8, mas ele carrega a interface de `mchose.com.cn` — e é no *bundle*
web que está a lógica de dispositivo, em JavaScript legível.

O descritor da interface HID declara quatro coleções vendor. Só duas têm uso
conhecido:

| Report ID | Usage Page | Tipo | Uso |
|---|---|---|---|
| `0x55` | `0xFF90` | Input + Output, 63 B | bateria |
| `0xAA` | `0xFF22` | Feature, 63 B | firmware |
| `0xED` | `0xFF21` | In/Out 1 B + Feature 15 B | desconhecido |
| `0x41` | `0xFF82` | Input + Output, 63 B | desconhecido |

O resto do descritor são teclas de mídia (report `0x06`) e telefonia (`0x07`),
que já funcionam como HID padrão — e que chegam no **mesmo** `/dev/hidraw`, então
quem lê precisa descartá-las em silêncio em vez de tratá-las como anomalia.

> [!CAUTION]
> Escrever nos report IDs `0xED` e `0x41` sem saber o que significam é caminho
> conhecido para brickar dispositivo. Este projeto não expõe nenhum construtor
> que os alcance, de propósito.

**Bateria.** Output report `0x55`, payload `[0x65, 0x01, 0x00 ×61]`. A resposta
chega como input report `0x55` começando com `0x65`:

```
escreve  [0x55, 0x65, 0x01, 0x00...]        (64 B)
lê       [0x55, 0x65, <bateria %>, <status>, ...]
```

Status: `2` descarregando · `3` carregando · `4` cheia · `26` dormindo.

**Firmware.** Feature report `0xAA` com `[0x01, <1=dongle|0=fone>, 0...]`, espera
de 300 ms, e leitura do mesmo feature. A resposta começa com `[170, 1]` e a
versão são os bytes 2 a 5.

O fone **empurra** a atualização de bateria sozinho, então o modo normal de
operação é passivo: sem polling.

> [!TIP]
> Para reencontrar o código no *bundle*: o chunk que importa passa de 7 MB e os
> hashes do nome rotacionam. Baixe conferindo o `Content-Length` — um download
> truncado silenciosamente já custou uma conclusão errada aqui, a de que o código
> deste fone não estava no *bundle* web.

## Armadilhas que custaram caro

**Existem duas famílias MCHOSE com protocolos diferentes.** `cmedia` (VID
`0x291d`) e `thx` (VID `0x3837` — V9 PRO 2, R9, X9, K9). A tabela `EQ_HID_CMD`
que aparece no *bundle* web pertence à segunda e **não** vale para o V9 PRO. A
função de bateria, essa, é comum às duas.

**O VID/PID não identifica o produto.** `291d:385d` é compartilhado por S9 PRO,
G9 PRO, V9 e V9 PRO — o chip C-Media reporta os mesmos IDs. Casar só por eles faz
o software falar com o fone errado.

**"Carregando" é praticamente inalcançável.** O fone se desconecta do dongle
2.4GHz quando é plugado para carregar. O estado existe no protocolo e quase nunca
chega: sumir e estar desligado são indistinguíveis do lado do Linux.

**EQ, surround 7.1 e redução de ruído não estão no firmware.** No Windows são
APOs — processamento de áudio rodando no host. No Linux o equivalente é PipeWire,
não protocolo de dispositivo.

## Desenvolvimento

```
crates/mchose-protocol/   bytes ↔ tipos, sem I/O, sem dependências
crates/mchose-device/     hidraw, hotplug, Stream de eventos
cosmic-applet-mchose/     o applet; state.rs não conhece libcosmic
install/                  regra udev, desktop entry, script de instalação
spikes/                   probe em Python que validou o protocolo
```

Para ver os eventos sem painel:

```bash
cargo run --locked -p mchose-device --example eventos
```

Testes usam bytes capturados do hardware, não inventados. Os marcados
`#[ignore]` falam com o fone de verdade:

```bash
cargo test --locked                                  # sem hardware
cargo test --locked -p mchose-device -- --ignored    # precisa do dongle plugado
```

> [!IMPORTANT]
> Use sempre `--locked`. Sete dependências vêm de branch móvel do `pop-os`, sem
> commit fixado no manifesto — entre elas o `cosmic-config-derive`, que é
> proc-macro e **executa em tempo de compilação**. Só o `Cargo.lock` fecha a
> árvore; sem ele, uma re-resolução puxa código novo que roda como você.

## Referências

- [HeadsetControl](https://github.com/Sapd/HeadsetControl) — o
  [PR #563](https://github.com/Sapd/HeadsetControl/pull/563) adiciona o MCHOSE X9
  e usa o mesmo `55 65 01 00` para bateria, apesar de ser da outra família.
- [cosmic-applets](https://github.com/pop-os/cosmic-applets) — o padrão de
  `Subscription` que o applet vai seguir.
