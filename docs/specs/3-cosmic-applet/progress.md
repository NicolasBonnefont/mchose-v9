# Progress — 3 cosmic-applet-mchose

Task 1: completa — reexports de BatteryReading/ChargeState/FirmwareVersion,
exemplo `eventos` sem bytes crus, e as 4 referencias a regra `99-` (a que nunca
funcionou) corrigidas para `72-` no codigo e na documentacao.

O exemplo rodou contra o hardware e provou a cadeia pela API publica:
  conectado — firmware do dongle 0012, do fone 0036
  bateria 70% — Discharging

Antes disso: o build do libcosmic falhou por falta de libxkbcommon-dev e mais 17
bibliotecas de sistema. Instaladas, compila (470 rlibs). Eu tinha reportado "exit
0" numa primeira tentativa por ter canalizado o cargo por um `tail` — o codigo de
saida que voltou era o do tail.

Task 2: completa — 10 testes no modulo de estado, que nao conhece libcosmic. A
fronteira que qualidade exigiu e o que torna as sete frases verificaveis.

Descoberta no compilador: BatteryReading e FirmwareVersion sao #[non_exhaustive],
o que impede literal de struct fora do crate que os define — reexportar nao
bastava, o applet nao conseguia construi-los nem em teste. Ganharam construtores
`new`. Expansao de escopo para crates/mchose-protocol/**, declarada no spec.

Task 3: completa — o applet. A API de popup foi lida da fonte do libcosmic no rev
fixado (`examples/applet/src/window.rs`), nao de memoria nem do doc: o doc manda
declarar `cosmic = { version = "1.0" }` do crates.io, que e um build tool de C/C++
sem relacao com o projeto.

Task 4: completa — desktop entry com Exec absoluto e script de instalacao.
`desktop-file-validate` reclama de `Categories=COSMIC;` por nao ser categoria
registrada no freedesktop; e o que os applets oficiais usam, entao consistencia
com a plataforma vence.

VERIFICADO NO PAINEL (11/09/2026): instalado e o painel reiniciado, o applet
aparece no canto inferior esquerdo mostrando o icone de fone e "70%" — a bateria
real do dispositivo. Capturado com cosmic-screenshot; o grim nao serve porque o
cosmic-comp nao implementa wlr-screencopy.
