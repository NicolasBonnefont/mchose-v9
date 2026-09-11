# Tasks — 1 mchose-protocol: pacotes HID de bateria e firmware

- [x] ### 1. Workspace, crate e portões de lint
  **Arquivos:** Criar: `Cargo.toml`, `rust-toolchain.toml`, `crates/mchose-protocol/Cargo.toml`, `crates/mchose-protocol/src/lib.rs` · Editar: `.gitignore` · Teste: —
  **Consumes:** nada; é a primeira task
  **Produces:** workspace de um membro, toolchain fixada, `#![forbid(unsafe_code)]` e os lints de `clippy` ativos no crate
  **Requisito:** invariantes "o crate é `#![forbid(unsafe_code)]` e sem dependências fora da `std`" e "nenhuma entrada causa pânico" (os lints são o que sustenta a segunda); `Arquivos no escopo`
  **Verificação:** `cargo clippy -p mchose-protocol -- -D warnings -D clippy::indexing_slicing -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic && cargo fmt --check && test -f Cargo.lock`

- [x] ### 2. Identificação do dispositivo
  **Arquivos:** Criar: `crates/mchose-protocol/src/device.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** crate da task 1
  **Produces:** predicado de reconhecimento por VID/PID e nome
  **Requisito:** `Comportamento alvo` → "Identificação do dispositivo"; invariante "a identificação casa as duas formas do nome que o kernel expõe"
  **Verificação:** `cargo test -p mchose-protocol device::` — cobre as duas formas do nome, `V9 PRO 2`, `V9 PRO 2 ULTRA`, outro produto, nome vazio e nome de 4096 bytes

- [ ] ### 3. Constantes de tempo e construção dos requests
  **Arquivos:** Criar: `crates/mchose-protocol/src/request.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** crate da task 1
  **Produces:** os dois construtores fechados (bateria, firmware dongle/fone) e as constantes de 300 ms e 2 s
  **Requisito:** `Comportamento alvo` → "Bateria", "Firmware", "Constantes de tempo"; invariantes "a API de request é fechada" e "todo buffer de request tem exatamente 64 bytes"
  **Verificação:** `cargo test -p mchose-protocol request::` — `len() == 64`, prefixos `[0x55,0x65,0x01]`, `[0xAA,0x01,0x01]` e `[0xAA,0x01,0x00]`, resto zerado

- [ ] ### 4. Decodificação da resposta de bateria
  **Arquivos:** Criar: `crates/mchose-protocol/src/battery.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** tipos da task 3
  **Produces:** decodificador de `0x55`/`0x65`, estado de carga e a falha única que carrega os bytes
  **Requisito:** `Comportamento alvo` → "Estado de carga", "Postura diante do inesperado", "Tráfego alheio é descartado em silêncio", "as três são a mesma coisa para quem consome"
  **Verificação:** `cargo test -p mchose-protocol battery::` — `55 65 46 02` → 70%/descarregando; status desconhecido preserva o percentual; 200 invalida e devolve os bytes; buffers de 0 a 3 bytes; 128 bytes; buffer sem report ID rejeitado; `0x06` descartado

- [ ] ### 5. Decodificação da resposta de firmware
  **Arquivos:** Criar: `crates/mchose-protocol/src/firmware.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** tipos da task 3
  **Produces:** decodificador do feature `0xAA` com os quatro bytes crus e a forma textual derivada
  **Requisito:** `Comportamento alvo` → "Firmware"; invariante "a forma textual da versão é só exibição"
  **Verificação:** `cargo test -p mchose-protocol firmware::` — `aa 01 00 00 01 02 ff 25` → `"0012"`; `aa 01 00 00 03 06 ff 25` → `"0036"`; buffers de 0 a 5 bytes; buffer sem report ID rejeitado

- [ ] ### 6. Module Pack e atualização do AGENTS.md da raiz
  **Arquivos:** Criar: `crates/mchose-protocol/AGENTS.md` · Editar: `AGENTS.md` · Teste: —
  **Consumes:** o crate pronto das tasks 1 a 5
  **Produces:** o pack que os cards #2, #3 e #4 leem antes de consumir o crate; raiz descrevendo o repositório como ele ficou
  **Requisito:** `Arquivos no escopo` → `crates/mchose-protocol/AGENTS.md` e `AGENTS.md` (raiz), cujas seções *Stack real*, *Comandos* e *Rede de segurança automatizada* deixam de ser verdade com este card
  **Verificação:** `bash "$HOME/.claude/skills/sdd-validate/scripts/validate.sh" docs/specs/1-mchose-protocol master` e leitura: nenhuma afirmação de "não existe código/teste/build" sobrevive
