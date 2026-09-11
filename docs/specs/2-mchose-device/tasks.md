# Tasks — 2 mchose-device: acesso ao hidraw com hotplug

- [x] ### 1. Scaffold do crate e dependências
  **Arquivos:** Criar: `crates/mchose-device/Cargo.toml`, `crates/mchose-device/src/lib.rs` · Editar: `Cargo.toml` (raiz) · Teste: —
  **Consumes:** o workspace e o crate `mchose-protocol` do card #1
  **Produces:** segundo membro do workspace, com os mesmos lints antipânico e `forbid(unsafe_code)` onde couber
  **Requisito:** `Arquivos no escopo`; invariante "valem as invariantes do mchose-protocol", parte "nenhuma entrada causa pânico"
  **Verificação:** `cargo clippy -p mchose-device -- -D warnings -D clippy::indexing_slicing -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic && cargo fmt --check`

- [x] ### 2. Descoberta por sysfs
  **Arquivos:** Criar: `crates/mchose-device/src/discovery.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** `mchose_protocol::device::is_supported`
  **Produces:** enumeração que lê `HID_ID` e `HID_NAME` do `uevent` e devolve o caminho do `hidraw` que casou
  **Requisito:** `Comportamento alvo` → "Descoberta"; invariante "nada é aberto antes de `is_supported` devolver verdadeiro"
  **Verificação:** `cargo test -p mchose-device discovery::` — sobre uma árvore de `sysfs` falsa em diretório temporário: casa o V9 PRO, ignora irmão de mesmo VID/PID, ignora `uevent` malformado, e nenhum arquivo de dispositivo é aberto

- [ ] ### 3. Trait de transporte e o fake
  **Arquivos:** Criar: `crates/mchose-device/src/transport.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** tipos do `mchose-protocol`
  **Produces:** trait `pub(crate)` com quatro operações (ler, escrever, `GET_FEATURE`, `SET_FEATURE`) e um fake que encena respostas, silêncio, truncamento e sumiço
  **Requisito:** `Comportamento alvo` → "Transporte atrás de um trait"; invariante "o trait de transporte é `pub(crate)`"
  **Verificação:** `cargo test -p mchose-device transport::` — o fake devolve os pacotes capturados e encena as quatro situações

- [ ] ### 4. Máquina de estados e sequenciamento
  **Arquivos:** Criar: `crates/mchose-device/src/machine.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** o trait da task 3
  **Produces:** consulta na conexão, escuta do push, prazo de resposta, transições de estado, espera assíncrona do feature
  **Requisito:** `Comportamento alvo` → "Escuta passiva", "Estados observáveis"; invariantes do evento owned e do no-op da consulta sob demanda
  **Verificação:** `cargo test -p mchose-device machine::` — os sete critérios do fake no spec, incluindo consulta sob demanda em estado estacionário e no-op com dongle ausente

- [ ] ### 5. Transporte real sobre hidraw
  **Arquivos:** Criar: `crates/mchose-device/src/transport/hidraw.rs` · Editar: `src/transport.rs` · Teste: `#[ignore]` no próprio módulo
  **Consumes:** o trait da task 3
  **Produces:** implementação real com os dois ioctls de feature e espera assíncrona do descritor
  **Requisito:** `Comportamento alvo` → "Transporte atrás de um trait", parte da implementação real; `Critério de verificação` → caminho real
  **Verificação:** `cargo test -p mchose-device --no-run` e depois o binário sob privilégio com `--ignored`, com `uhid` carregado

- [ ] ### 6. Hotplug e o Stream público
  **Arquivos:** Criar: `crates/mchose-device/src/hotplug.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** tasks 2 a 5
  **Produces:** monitor criado antes da enumeração, laço que não termina quando o dongle some, `Stream` de eventos owned e a alça de consulta
  **Requisito:** `Comportamento alvo` → "Hotplug", "Fluxo de eventos"; invariante "o evento publicado é owned e `Clone + Send + 'static`"
  **Verificação:** `cargo test -p mchose-device` — dispositivo que some não encerra o fluxo, e que volta produz leitura sem reassinatura

- [ ] ### 7. Module Pack e AGENTS.md da raiz
  **Arquivos:** Criar: `crates/mchose-device/AGENTS.md` · Editar: `AGENTS.md` · Teste: —
  **Consumes:** o crate pronto
  **Produces:** o pack que o card #3 vai ler; raiz descrevendo dois crates, as dependências novas e os testes que exigem privilégio
  **Requisito:** `Arquivos no escopo` → `crates/mchose-device/AGENTS.md` e `AGENTS.md` (raiz)
  **Verificação:** `bash "$HOME/.claude/skills/sdd-validate/scripts/validate.sh"` e leitura: nenhuma afirmação de "workspace de um crate" ou "faltam headers" sobrevive
