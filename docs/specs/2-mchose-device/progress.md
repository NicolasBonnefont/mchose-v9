# Progress — 2 mchose-device

Task 1: completa — segundo membro do workspace, com tokio, futures, udev e libc.
`deny(unsafe_code)` em vez de `forbid`: os ioctls de feature nao tem involucro
seguro na std e o transporte real vai precisar abrir a excecao num modulo so.
