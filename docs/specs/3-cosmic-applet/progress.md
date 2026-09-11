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
