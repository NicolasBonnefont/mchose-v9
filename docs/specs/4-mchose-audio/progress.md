# Progress — 4 mchose-audio

SPIKE ANTES DO SPEC. O card se declarava o de maior risco e era o unico sem
spike; rodei antes de escrever. Achados: a filter-chain cria o sink virtual, o
sistema ja traz `sink-eq6.conf` como template, e mudar ganho ao vivo funciona.
Testei tambem se o locale pt_BR quebrava o parse de float — nao era isso.

Task 1-2: scaffold e geracao pura. 11 testes, incluindo um por caractere
perigoso no nome do sink.
Task 3: camada do PipeWire. 8 testes sobre JSON gravado do pw-dump.
Task 4: escrita atomica com marcador. 8 testes, todos em diretorio temporario.
Task 5-6: exemplo, script e pack.

VERIFICADO NO SISTEMA REAL: o texto gerado carrega no PipeWire, o sink
`mchose_v9_eq` nasce, o `pw-link` mostra a saida ligada ao sink do fone, e as
dez bandas aplicam ganho de verdade.

DOIS BUGS QUE SO O SISTEMA REAL EXPOS:
- `?` dentro do laco abortava a busca no primeiro objeto do dump sem
  `info.props` — e o dump real comeca com Core, Client e Device. Com o dongle
  plugado, o exemplo dizia "sink nao encontrado".
- o microfone do mesmo fone tem `alsa.components` e `alsa.card_name` identicos;
  sem exigir `media.class = Audio/Sink`, o EQ sairia apontado para a entrada.

UMA CONCLUSAO DO SPIKE QUE SE MOSTROU ERRADA: escrevi no spec que "aplicar num
no suspenso devolve nao-aplicado". Sob o `filter-chain.service` do sistema a
escrita pega mesmo com o no suspenso; o comportamento que observei era de uma
instancia propria (`pipewire -c`). A invariante virou "o resultado vem da
releitura, nao do estado do no".

Minor: o surround saiu do card por depender de um HRIR com licenciamento que eu
nao consigo verificar. Vira card proprio.
Minor: `serde_json` e a unica dependencia externa; escrever parser de JSON a mao
seria superficie pior.
