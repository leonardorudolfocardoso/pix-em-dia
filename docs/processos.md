# Processos do Em Dia

Diagramas dos processos combinados antes do código (Pix com vencimento, baixa por consulta periódica, cobranças recorrentes e avulsas, lembretes enviados por uma pessoa). O GitHub renderiza os blocos `mermaid`.


## Mapa de contextos (proposta)

Cinco contextos dentro do Em Dia e três de fora. Cada contexto tem a própria linguagem: o que o sistema do cliente chama de "aluno" ou "plano" o Em Dia chama de "assinatura", e o que o Inter chama de "cobv CONCLUIDA" o Em Dia chama de "cobrança paga". As setas apontam de quem fornece (U, upstream) para quem depende (D, downstream).

```mermaid
flowchart LR
  subgraph fora["Fora do Em Dia"]
    direction TB
    cliente["Sistema do cliente<br/>ex.: cadastro de clientes, planos"]
    inter["Banco Inter<br/>API Pix (padrão Banco Central)"]
    canal["Canal humano<br/>WhatsApp de quem envia"]
  end
  subgraph emdia["Em Dia"]
    direction TB
    org["Organizações e acesso<br/>organização, token, credenciais,<br/>certificado"]
    cob["Cobrança (núcleo)<br/>assinatura, cobrança, período,<br/>chave de idempotência, situação"]
    rec["Recebimento<br/>pagamento, baixa, conciliação"]
    bco["Integração bancária<br/>adaptador por banco"]
    lem["Lembretes<br/>lista do dia, contato, mensagem"]
  end
  cliente -- "U → D · importação por<br/>referência externa" --> cob
  org -- "U → D · contexto autenticado<br/>(kernel compartilhado)" --> cob
  org -- "credenciais" --> bco
  inter -- "U → D · camada anticorrupção" --> bco
  bco -- "emitir, consultar, cancelar" --> cob
  bco -- "listar pagamentos" --> rec
  rec -- "marca como paga" --> cob
  cob -- "U → D · cobranças abertas" --> lem
  org -- "aviso do certificado" --> lem
  lem -- "lista pronta" --> canal
```

- **Cobrança** é o núcleo: decide o que cobrar, de quem e quando, e garante que nada duplica. É onde está o valor do produto.
- **Recebimento** transforma "um Pix chegou" em "esta cobrança está paga". Fica separado porque terá outras fontes depois (webhook, Pix Automático) sem mudar a Cobrança.
- **Integração bancária** é a camada anticorrupção: o formato do Inter não passa daqui. Um banco novo é um adaptador novo.
- **Organizações e acesso** é o kernel compartilhado: todo contexto recebe o contexto autenticado, e nenhum recebe a organização como parâmetro.
- **Lembretes** só lê cobranças. Nunca altera nada nelas e nunca envia mensagens sozinho.
- **Sistema do cliente** fica fora: turmas, chamada e cadastro de alunos não entram no Em Dia. A ligação é a `referencia_externa`. Mais tarde, uma API ou o servidor MCP seria o serviço aberto que o cliente usa.

Na v1, tudo roda num único programa com um banco de dados. Os contextos viram pastas no código (`cobranca/`, `recebimento/`, `bancos/`, `organizacoes/`, `lembretes/`), e um contexto só chama outro pela interface pública dele.


## Visão geral

Três rotinas agendadas falam com o banco só por meio do adaptador. Ninguém de fora chama o Em Dia: sem webhook, não existe servidor público.

```mermaid
flowchart LR
  planilha["Planilha ou sistema<br/>da organização"] -->|importa assinaturas| emdia
  subgraph emdia["Em Dia"]
    direction TB
    emitir["Rotina: emitir<br/>dia 1 do mês"]
    baixar["Rotina: baixar<br/>a cada 15 min"]
    lista["Rotina: lista do dia<br/>toda manhã"]
    db[("Banco de dados<br/>isolado por organização")]
    adaptador["Adaptador do banco"]
    emitir --> db
    baixar --> db
    lista --> db
    emitir --> adaptador
    baixar --> adaptador
  end
  adaptador -->|mTLS + token| inter["Banco Inter<br/>API Pix"]
  cofre[("Cofre de segredos<br/>certificado e chave")] -.-> adaptador
  lista --> pessoa["Pessoa da organização<br/>envia pelo WhatsApp"]
  pessoa --> familia["Quem paga"]
  familia -->|paga o Pix| inter
```


## Emissão mensal

Primeiro grava a cobrança no nosso banco, depois pede ao Inter. Como o `txid` é calculado, repetir a rotina sempre cai na mesma cobrança.

```mermaid
sequenceDiagram
  autonumber
  participant J as Rotina emitir
  participant D as Banco de dados
  participant A as Adaptador
  participant I as Inter
  J->>D: para cada assinatura ativa:<br/>grava cobrança (a_emitir, txid calculado)<br/>se já existir, não faz nada
  Note over D: já existe? nada muda.<br/>Uma por assinatura por período.
  J->>D: lista cobranças em a_emitir
  loop cada cobrança, até 100 por minuto
    J->>A: emitir_cobranca(txid, valor, vencimento)
    A->>I: PUT /pix/v2/cobv/{txid}
    alt criou ou já existia ativa
      I-->>A: 201 com location / copia e cola
      A-->>J: cobrança aberta
      J->>D: situação = aberta
    else 429 (muitas chamadas)
      I-->>A: 429
      A->>A: espera e tenta de novo
    else dados recusados
      I-->>A: 400
      A-->>J: erro do banco
      J->>D: situação = falhou + motivo
    end
  end
  Note over J,I: Se o processo cair entre o passo 5 e o 7,<br/>a próxima execução repete o PUT com o mesmo txid<br/>e só completa a gravação.
```


## Cobrança avulsa

Taxa de inscrição, material ou qualquer cobrança única. Quem cria informa a chave; pedir de novo com a mesma chave devolve a cobrança que já existe.

```mermaid
flowchart TD
  pedido["Pedido: Evento de dezembro, R$ 80,<br/>vence 10/12, para 25 assinaturas"] --> cada["Para cada assinatura"]
  cada --> chave["chave = evento-dez-2026:referência"]
  chave --> existe{"Já existe cobrança<br/>com essa chave?"}
  existe -->|sim| devolve["Devolve a existente<br/>sem criar outra"]
  existe -->|não| grava["Grava a_emitir<br/>contato copiado da assinatura"]
  grava --> emite["Emite no banco<br/>como na mensal"]
  emite --> aberta["aberta"]
```


## Baixa automática

A cada 15 minutos o Em Dia pergunta ao Inter o que foi pago, só para as organizações que têm cobrança aberta. O intervalo é fixo para manter o banco de dados na camada gratuita do Neon (veja [contexto](contexto.md)). A janela olha uma hora para trás, então um pagamento pode ser visto duas vezes, e isso não faz mal.

As cobranças pagas vêm de `GET /cobv`. Na transição, a rotina também lê `GET /pix` (escopo `pix.read`, temporário) e guarda só os Pix sem `txid`: quem ainda paga direto na chave. Esses ficam como pagamento sem cobrança até uma pessoa ligá-los à cobrança certa na lista do dia. Sem `pix.read`, esse passo é pulado e o resto funciona igual.

```mermaid
sequenceDiagram
  participant J as Rotina baixar
  participant A as Adaptador
  participant I as Inter
  participant D as Banco de dados
  J->>A: listar_pagamentos(desde a última execução - 1h)
  A->>I: GET /cobv concluídas
  I-->>A: pagamentos com txid, valor, endToEndId
  opt pix.read na transição
    A->>I: GET /pix
    I-->>A: Pix recebidos; fica só quem não tem txid
  end
  A-->>J: lista em tipos nossos
  loop cada pagamento
    J->>D: acha cobrança pelo txid
    alt encontrou e valor confere
      J->>D: grava pagamento (único por endToEndId)
      J->>D: situação = paga
    else sem txid, txid desconhecido ou valor diferente
      J->>D: grava pagamento sem cobrança (sem dono), sem baixa
    end
  end
  J->>D: abertas com vencimento + dias após vencimento no passado:<br/>situação = expirada
  Note over J,D: Uma vez por dia a mesma rotina<br/>varre os últimos 7 dias, como rede de segurança.
```


## Lista diária de lembretes

O Em Dia monta a lista e a mensagem; uma pessoa envia. Antes de montar, roda uma baixa extra para não lembrar quem acabou de pagar.

```mermaid
flowchart TD
  inicio(["Toda manhã"]) --> baixa["Roda uma baixa na hora"]
  baixa --> seleciona["Seleciona cobranças abertas:<br/>vencem em até 3 dias ou já venceram"]
  seleciona --> agrupa["Agrupa pelo telefone de contato<br/>avô de 3 netos = 1 mensagem"]
  agrupa --> monta["Monta a mensagem fixa, sem IA:<br/>descrição, valor, vencimento, Pix copia e cola<br/>+ Se você já pagou, está tudo certo"]
  monta --> alertas["Coloca no topo:<br/>falhas, Pix sem cobrança (nome, valor, hora),<br/>aviso do certificado"]
  alertas --> entrega["Entrega a lista"]
  entrega --> humano["Pessoa revisa e envia pelo WhatsApp"]
```

Antes de enviar, a pessoa liga cada Pix sem cobrança à cobrança certa ("dar baixa à mão"), e quem já pagou sai da lista. A mesma ação serve para pagamento em dinheiro.

Onde a lista aparece é decidido no item E0.2 do [roteiro](roteiro.md).


## Ciclo de vida de uma cobrança

As situações são nossas, não as do Inter. "Atrasada" não é uma situação: é uma cobrança aberta com o vencimento no passado.

```mermaid
stateDiagram-v2
  [*] --> a_emitir: gravada pela rotina<br/>ou pedido avulso
  a_emitir --> aberta: Inter criou
  a_emitir --> falhou: Inter recusou
  falhou --> a_emitir: cadastro corrigido
  a_emitir --> cancelada: cancelada antes de ir ao banco
  falhou --> cancelada: alguém desistiu
  aberta --> paga: baixa encontrou o Pix<br/>ou pessoa deu baixa à mão
  aberta --> cancelada: alguém cancelou
  aberta --> expirada: rotina baixar viu que passou<br/>o vencimento + dias após vencimento
  paga --> [*]
  cancelada --> [*]
  expirada --> [*]
```


## Modelo de dados

Quatro tabelas, nenhuma com termos de um cliente específico. Pagamentos que não batem com nenhuma cobrança ficam em `pagamentos` com `cobranca_id` vazio, para o relatório e os alertas. Toda tabela de dados tem `organizacao_id` e é protegida pelo isolamento por organização no próprio banco de dados, com Row Level Security do PostgreSQL.

```mermaid
erDiagram
  ORGANIZACOES ||--o{ ASSINATURAS : tem
  ORGANIZACOES ||--o{ COBRANCAS : recebe
  ASSINATURAS ||--o{ COBRANCAS : "gera (recorrente) ou empresta contato (avulsa)"
  COBRANCAS |o--o{ PAGAMENTOS : "quitada por (sem cobrança = pagamento sem dono)"
  ORGANIZACOES {
    uuid id
    text nome
    text hash_token
    text chave_pix
    smallint dias_apos_vencimento
    date certificado_vence_em
    timestamptz ultima_baixa_em
  }
  ASSINATURAS {
    uuid id
    text descricao
    text referencia_externa
    bigint valor_centavos
    smallint dia_vencimento
    boolean ativa
    text contato_nome
    text contato_telefone
  }
  COBRANCAS {
    uuid id
    uuid assinatura_id "opcional na avulsa"
    text tipo "recorrente ou avulsa"
    date periodo "só recorrente"
    text descricao
    text chave_idempotencia "única por organização"
    bigint valor_centavos
    date vencimento
    text situacao
    text motivo_falha "quando falhou"
    text txid "único por organização"
    text pix_copia_e_cola
  }
  PAGAMENTOS {
    uuid id
    uuid cobranca_id "vazio = pagamento sem dono"
    text txid "como veio do banco; vazio = Pix direto na chave"
    text pagador_nome "só sem cobrança, se o banco informar"
    bigint valor_centavos
    timestamptz pago_em
    text banco_e2e_id "único: mesmo Pix, um pagamento"
  }
```


## Certificado anual

O certificado do Inter vale 1 ano em produção (30 dias no sandbox). A renovação fica liberada 90 dias antes e mantém client id e secret; só troca os arquivos.

```mermaid
flowchart LR
  c90["90 dias antes<br/>1º aviso: renovação liberada"] --> c30["30 dias<br/>2º aviso"]
  c30 --> c14["14 dias<br/>3º aviso"]
  c14 --> c7["7 dias<br/>aviso diário"]
  c7 --> renova["Responsável renova no Internet Banking<br/>e envia os novos .crt e .key"]
  renova --> cofre["Novos arquivos no cofre<br/>Em Dia segue sem parar"]
```
