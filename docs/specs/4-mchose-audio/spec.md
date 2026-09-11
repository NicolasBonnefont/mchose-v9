# 4 — mchose-audio: EQ via PipeWire

**Card:** https://github.com/NicolasBonnefont/mchose-v9/issues/4
**Módulo:** `crates/mchose-audio`
**Status:** rascunho

> Escrito sem entrevista, por decisão do autor de rodar os cards restantes sem
> interação. As escolhas que teriam virado pergunta estão marcadas como
> **premissa**. Este card teve **spike executado antes do spec** — era o único
> que nunca tivera, e o próprio card se declarava o de maior risco.

## Comportamento atual

Não existe nada de áudio no repositório. O fone aparece como sink ALSA comum
(`alsa_output.usb-C-Media_Electronics_Inc_MCHOSE_V9_PRO_…`), em s24le 2ch 48 kHz,
e nada processa o som.

O que o spike apurou, executando de verdade nesta máquina:

- **`libpipewire-module-filter-chain.so` está presente**, e o sistema traz
  templates prontos em `/usr/share/pipewire/filter-chain/` — entre eles o
  `sink-eq6.conf`, um EQ de 6 bandas com `bq_lowshelf`, `bq_peaking` e
  `bq_highshelf` encadeados. Estender para 10 bandas é acrescentar nós e links.
- **Um sink virtual carregado com `pipewire -c <conf>` aparece na hora** e recebe
  áudio como qualquer outro.
- **Mudar ganho em tempo de execução funciona**, via o parâmetro `Props` do nó:
  `b2:Gain` foi de `0.0` para `6.0` e o valor persistiu.
- **Mas só com o sink ativo.** Com o sink suspenso, o mesmo comando é aceito,
  não devolve erro, e **não tem efeito nenhum**. Foi preciso tocar áudio para o
  filtro sair de `SUSPENDED` antes de o valor pegar. Essa é a descoberta que
  muda o desenho: o crate não pode confiar em "mandei, então aplicou".
- **O surround 7.1 depende de um arquivo que não temos.**
  `sink-virtual-surround-7.1-hesuvi.conf` exige `hrir_hesuvi/hrir.wav`, ausente
  na máquina. É resposta impulsional com licenciamento próprio.

## Comportamento alvo

Um crate `mchose-audio` que oferece um EQ de 10 bandas para o fone.

**Geração da configuração.** Uma função pura recebe os ganhos **e o `node.name`
do sink alvo**, e devolve o texto da `filter-chain`: dez nós — o primeiro
`bq_lowshelf`, o último `bq_highshelf`, os oito do meio `bq_peaking` — em série,
mais `target.object` em `playback.props`.

O alvo não é enfeite: o `sink-eq6.conf` do sistema **não** tem `target.object`, e
por isso o sink virtual dele segue o sink *padrão*. Sem esse campo o EQ
processaria o áudio de qualquer saída, não o do fone.

> **Premissa:** frequências centrais em `31, 62, 125, 250, 500, 1k, 2k, 4k, 8k,
> 16k` Hz — a série de oitavas de todo EQ gráfico de 10 bandas — com `Q = 1.41`,
> que é a largura de uma oitava. Ganho em `-12..=+12` dB. **Escolha nossa, sem
> dado do fabricante:** a tabela `EQ_HID_CMD` do bundle é da família `thx` e não
> vale para este fone, como o próprio README registra.

**Achar o sink do fone.** Pelo `mchose_protocol::device::is_supported`,
alimentado com `alsa.components` e `alsa.card_name` do nó — o sink expõe
`USB291d:385d` e `MCHOSE V9 PRO`. Nunca por VID/PID sozinho nem por prefixo de
nome: `291d:385d` é compartilhado com S9 PRO, G9 PRO e V9, e prefixo casaria
`V9 PRO 2`, que o pack do protocolo manda excluir antes de incluir.

**Instalar não é aplicar.** Escrever o arquivo não cria o sink: ele só nasce
depois de `systemctl --user restart filter-chain.service`. E um texto recusado
vira **aviso no log**, não erro — verificado: um fragmento malformado produz
`error in config … Mismatched bracket`, os outros carregam, o nosso some, e
ninguém é avisado. O crate confirma que o nó existe depois de instalar, em vez
de assumir.

**Aplicação ao vivo.** Os ganhos são escritos no parâmetro `Props` do nó, sem
recarregar nada — e o arquivo é atualizado junto.

**O arquivo é a fonte da verdade do ganho.** Toda escrita ao vivo bem-sucedida
atualiza o `control` correspondente no `.conf`; sem isso o próximo restart
reverte o EQ em silêncio.

**Nunca assumir que aplicou; confirmar relendo.** No spike, com a filter-chain
hospedada numa instância própria (`pipewire -c`), escrever ganho num nó suspenso
foi aceito e **silenciosamente ignorado**. Já sob o `filter-chain.service` do
sistema, a mesma escrita pega mesmo com o nó suspenso. O comportamento varia com
a hospedagem, então o crate não deduz nada do estado: escreve, relê `Props`, e
responde o que a releitura disser. A releitura vem **depois** da escrita, nunca
antes — entre checar e escrever o nó pode mudar de estado.

**Ler os ganhos atuais** também faz parte da API, e o `node.name` do sink virtual
é estável e público — sem as duas coisas, os sliders de um futuro card de UI
nasceriam sem valor inicial.

## Invariantes

- **A geração da configuração não faz I/O.** É função pura, testável sem
  PipeWire — mesmo padrão do `mchose-protocol`.
- **Nenhum valor de origem externa é interpolado no texto gerado.** Frequências,
  Q e tipos de nó são constantes do crate; o ganho entra formatado como número
  depois de recusar NaN e infinito; **o nome do sink alvo só é aceito se casar
  `^[A-Za-z0-9._:-]+$`, e é recusado, nunca sanitizado.** O nome vem das strings
  USB que o próprio dispositivo declara — `manufacturer`, `product` e `serial` —
  e um `x" } plugin = "/tmp/e.so` fecharia o bloco SPA-JSON e acrescentaria uma
  diretiva num arquivo que o PipeWire carrega no login, resolvendo `plugin` para
  um `.so` no processo da sessão.
- **O arquivo tem nome constante e destino fixo.** `mchose-v9-eq.conf`, escrito
  só dentro de `$XDG_CONFIG_HOME/pipewire/filter-chain.conf.d/`, nunca em caminho
  derivado de entrada — `~/.config/` é diretório de execução, e nome derivado
  permitiria travessia para `autostart/`. Escrita atômica (tmp + rename).
- **Arquivo preexistente sem o cabeçalho-marcador do crate não é sobrescrito.**
  A instalação falha e diz qual arquivo bloqueou; `effect_input.eq6` e vizinhos
  são do usuário, e o crate nunca remove o que não escreveu.
- **O serial não aparece no texto gerado nem em mensagem de erro.** A ligação ao
  sink físico é feita em runtime; o arquivo referencia o `node.name`, e a mesma
  regra do `PermissionDenied` do applet vale aqui — isto é feito para ser colado
  em issue.
- **Ganho fora de `-12..=+12` dB não é escrito.**
- **O resultado vem da releitura, não do estado do nó.** "Suspenso implica não
  aplicado" **não** é regra: foi observado numa instância própria de PipeWire e
  não se reproduz sob o `filter-chain.service`. O crate reporta `Aplicado` ou
  `NaoAplicado` conforme o valor relido, e o campo `ativo` do nó é informativo,
  não preditivo.
- **O crate não conhece HID nem o `mchose-device`.** Depende do
  `mchose-protocol` só por `is_supported`, que é a regra de identificação do
  projeto e não pode ser reimplementada aqui.
- **Nada de shell e nada de FFI.** A conversa com o PipeWire é por processo
  externo com `argv` — sem `sh -c`, sem interpolação — e o nó é endereçado por
  **id numérico**, nunca por nome. Isso mantém o `unsafe` do projeto confinado ao
  `mchose-device` e fecha o vetor de injeção de comando que um nome com aspas
  abriria.
- **Nenhuma entrada causa pânico.** Mesma disciplina e mesmos lints dos outros
  crates.

## Fora de escopo

- **Surround e virtualização 7.1.** Depende de `hrir_hesuvi/hrir.wav`, que não
  existe nesta máquina e tem licenciamento próprio; embarcar um arquivo de
  procedência não verificada num repositório público é decisão de outra ordem.
  Vira card próprio, com a pergunta do HRIR explícita. O `sink-dolby-surround`
  do sistema não substitui: ele decodifica conteúdo matrizado, não virtualiza
  canais em fones. O `sink-virtual-surround-5.1-kemar.conf` tem a mesma
  dependência. Quando o surround virar card, o HRIR será apontado a partir de
  arquivo **já instalado pelo usuário**; baixar em runtime está fora de questão.
- **Interface do EQ no applet.** Este card entrega o crate e um
  `examples/eq.rs`, que lê ganhos da linha de comando e os aplica — é o consumidor
  real, e o caminho que o teste `#[ignore]` exercita. Pôr sliders no popover é
  card próprio.
- **Instalar o arquivo numa máquina e reiniciar o serviço.** É do
  `install/install-eq.sh`, no mesmo padrão do `install-applet.sh` e da regra
  udev. O crate gera o texto, escreve no destino fixo e confirma o nó; quem
  reinicia o serviço é o script.
- **Presets de EQ.** Nem os do fabricante, que estão na família `thx` e não se
  aplicam a este fone.
- **Redução de ruído do microfone.** O `source-rnnoise.conf` existe no sistema,
  mas é outro caminho de sinal e outro card.
- **Trocar o sink padrão do sistema** para o virtual. Efeito colateral na sessão
  do usuário; quem decide é ele.

## Arquivos no escopo

- `Cargo.toml`
- `Cargo.lock`
- `crates/mchose-audio/**`
- `install/**`
- `spikes/**`
- `AGENTS.md`
- `README.md`
- `docs/specs/4-mchose-audio/**`

## Critério de verificação

```bash
cargo test --locked -p mchose-audio
cargo clippy --locked -p mchose-audio -- -D warnings -D clippy::indexing_slicing \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
cargo fmt --check
```

Sobre a geração, sem PipeWire nenhum:

- dez bandas produzem dez nós e nove links
- a primeira é `bq_lowshelf`, a última `bq_highshelf`, as oito do meio `bq_peaking`
- as frequências saem na ordem declarada
- ganho fora de `-12..=+12`, NaN e infinito são recusados
- nome de sink com `"`, `}`, `#` ou quebra de linha é recusado — um teste por caso
- o texto gerado contém `target.object` com o nome do alvo
- os testes escrevem em diretório temporário injetado; nenhum teste, nem os
  `--ignored`, toca o `~/.config` real

Caminho real, sob demanda e com o PipeWire da sessão:

```bash
cargo test --locked -p mchose-audio -- --ignored
```

- o sink virtual aparece depois de carregada a configuração
- escrever ganho com o sink **ativo** muda o valor lido de volta
- o resultado relatado bate com o valor relido, com o sink ativo **e** suspenso
- instalar texto inválido faz o crate reportar falha, não sucesso
- escrita ao vivo bem-sucedida também atualiza o `control` no arquivo
