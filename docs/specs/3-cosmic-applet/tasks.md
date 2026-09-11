# Tasks — 3 cosmic-applet-mchose: bateria no painel

- [x] ### 1. Reexports, exemplo e a regra `99-` que ficou para trás
  **Arquivos:** Editar: `crates/mchose-device/src/lib.rs`, `crates/mchose-device/src/machine.rs`, `crates/mchose-device/src/transport/hidraw.rs`, `crates/mchose-device/AGENTS.md`, `AGENTS.md` · Criar: `crates/mchose-device/examples/eventos.rs` · Teste: —
  **Consumes:** o `mchose-device` como está
  **Produces:** `BatteryReading`, `ChargeState` e `FirmwareVersion` reexportados; exemplo que imprime o `Stream` sem bytes crus; referências à regra apontando para `72-`
  **Requisito:** `Arquivos no escopo` → o parágrafo do `lib.rs`; invariante "nem a UI nem exemplo algum imprimem bytes crus"; tabela de estados, que nomeia `72-mchose-v9.rules`
  **Verificação:** `cargo build --locked --examples` e `grep -r '99-mchose' --include='*.rs' --include='*.md'` sem resultado fora de `.arquivo/`

- [x] ### 2. O estado do applet, sem libcosmic
  **Arquivos:** Criar: `cosmic-applet-mchose/Cargo.toml`, `cosmic-applet-mchose/src/state.rs`, `cosmic-applet-mchose/src/lib.rs` · Editar: `Cargo.toml` · Teste: no próprio módulo
  **Consumes:** os tipos reexportados da task 1
  **Produces:** o módulo que recebe `DeviceEvent` e devolve o que mostrar — frase, percentual, firmware — sem nenhuma dependência de UI
  **Requisito:** invariante "o estado do applet é um módulo sem libcosmic"; a tabela de sete estados; o braço `_` do `non_exhaustive`
  **Verificação:** `cargo test -p cosmic-applet-mchose --locked` — as sete frases, o último percentual sobrevivendo, `Connected` com `None` não apagando versão conhecida, e nenhum byte cru na saída

- [x] ### 3. O applet: assinatura, painel e popover
  **Arquivos:** Criar: `cosmic-applet-mchose/src/app.rs`, `cosmic-applet-mchose/src/main.rs` · Editar: `cosmic-applet-mchose/Cargo.toml`, `src/lib.rs` · Teste: —
  **Consumes:** o módulo de estado da task 2
  **Produces:** o binário do painel — ícone com percentual, popover com estado e firmware, `refresh()` ao abrir, e nova tentativa visível quando o fluxo acaba
  **Requisito:** `Comportamento alvo` → "A assinatura", "No painel", "No popover", "O fluxo pode acabar"
  **Verificação:** `cargo build -p cosmic-applet-mchose --locked` e `cargo clippy` com os quatro lints

- [x] ### 4. Instalação: desktop entry e script
  **Arquivos:** Criar: `install/com.github.NicolasBonnefont.mchose-v9.Applet.desktop`, `install/install-applet.sh` · Editar: `README.md` · Teste: —
  **Consumes:** o binário da task 3
  **Produces:** o desktop entry com `X-CosmicApplet=true` e `Exec=` absoluto, e o script que instala e acrescenta o ID ao `plugins_wings`
  **Requisito:** `Comportamento alvo` → "O desktop entry"
  **Verificação:** `desktop-file-validate` no arquivo, e leitura: `Exec=` com caminho absoluto

- [x] ### 5. Module Pack e documentação
  **Arquivos:** Criar: `cosmic-applet-mchose/AGENTS.md` · Editar: `AGENTS.md`, `README.md`, `crates/mchose-device/AGENTS.md` · Teste: —
  **Consumes:** o applet pronto
  **Produces:** o pack do applet; blast radius do device com consumidor real; README com o estado atualizado
  **Requisito:** `Arquivos no escopo`
  **Verificação:** `bash "$HOME/.claude/skills/sdd-validate/scripts/validate.sh"` e leitura: nenhuma afirmação de "applet não começou" sobrevive
