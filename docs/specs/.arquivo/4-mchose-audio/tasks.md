# Tasks — 4 mchose-audio: EQ via PipeWire

- [x] ### 1. Scaffold do crate
  **Arquivos:** Criar: `crates/mchose-audio/Cargo.toml`, `crates/mchose-audio/src/lib.rs` · Editar: `Cargo.toml` · Teste: —
  **Consumes:** o workspace e o `mchose-protocol`
  **Produces:** quarto membro, com os mesmos lints antipânico e `forbid(unsafe_code)` — aqui cabe `forbid`, porque não há ioctl
  **Requisito:** invariante "nada de shell e nada de FFI"; `Arquivos no escopo`
  **Verificação:** `cargo clippy --locked -p mchose-audio -- -D warnings -D clippy::indexing_slicing -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic && cargo fmt --check`

- [x] ### 2. Geração da filter-chain, pura
  **Arquivos:** Criar: `crates/mchose-audio/src/config.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** nada além da std
  **Produces:** função que recebe dez ganhos e o `node.name` do alvo e devolve o texto; validação de faixa, de NaN/infinito e do nome
  **Requisito:** `Comportamento alvo` → "Geração da configuração"; invariantes "nenhum valor de origem externa é interpolado" e "ganho fora de `-12..=+12` não é escrito"
  **Verificação:** `cargo test --locked -p mchose-audio config::` — dez nós e nove links, `target.object` presente, e um teste por caractere perigoso no nome

- [x] ### 3. Falar com o PipeWire: achar o nó, ler e aplicar ganho
  **Arquivos:** Criar: `crates/mchose-audio/src/pipewire.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo (parse) e `#[ignore]` (real)
  **Consumes:** `mchose_protocol::device::is_supported`
  **Produces:** descoberta do sink por `alsa.components` + `alsa.card_name`, leitura dos ganhos, aplicação por id numérico, e a releitura que confirma
  **Requisito:** `Comportamento alvo` → "Achar o sink do fone", "Aplicação ao vivo", "O estado suspenso"; invariante do `argv` sem shell
  **Verificação:** `cargo test --locked -p mchose-audio pipewire::` sobre JSON de `pw-dump` gravado, mais os `#[ignore]` contra o PipeWire da sessão

- [x] ### 4. Persistência: o arquivo é a fonte da verdade
  **Arquivos:** Criar: `crates/mchose-audio/src/install.rs` · Editar: `src/lib.rs` · Teste: no próprio módulo
  **Consumes:** a geração da task 2
  **Produces:** escrita atômica com cabeçalho-marcador, recusa de arquivo alheio, e atualização do `control` a cada ganho aplicado
  **Requisito:** invariantes "o arquivo tem nome constante e destino fixo", "arquivo preexistente sem o marcador não é sobrescrito", "o arquivo é a fonte da verdade do ganho"
  **Verificação:** `cargo test --locked -p mchose-audio install::` — tudo em diretório temporário injetado, nenhum teste toca `~/.config`

- [x] ### 5. Script de instalação e exemplo
  **Arquivos:** Criar: `install/install-eq.sh`, `crates/mchose-audio/examples/eq.rs` · Teste: —
  **Consumes:** o crate pronto
  **Produces:** o script que instala e reinicia o serviço, e o exemplo que é o consumidor real
  **Requisito:** `Fora de escopo` → "instalar o arquivo numa máquina"; `Comportamento alvo` → o exemplo
  **Verificação:** `bash -n install/install-eq.sh` e `cargo build --locked --examples -p mchose-audio`

- [x] ### 6. Module Pack e documentação
  **Arquivos:** Criar: `crates/mchose-audio/AGENTS.md` · Editar: `AGENTS.md`, `README.md` · Teste: —
  **Consumes:** o crate pronto
  **Produces:** o pack com as invariantes ancoradas; README e constituição com quatro crates
  **Requisito:** `Arquivos no escopo`
  **Verificação:** `bash "$HOME/.claude/skills/sdd-validate/scripts/validate.sh"` e leitura: nenhuma afirmação de "falta o EQ" sobrevive
