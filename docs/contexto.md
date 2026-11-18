# Contexto e decisões

Por que o Em Dia é como é. Registro das decisões tomadas antes do código, com data, e dos fatos da API do banco em que elas se apoiam.

## Problema

Pequenos negócios com cobrança recorrente (o piloto é uma escolinha de esportes com cerca de 140 assinaturas de R$ 125 por mês) recebem Pix soltos na conta PJ e conferem à mão contra uma lista. Quem paga muitas vezes não é o titular (avô, tio), e uma pessoa pode pagar por várias assinaturas (irmãos). Além das mensalidades, existem cobranças pontuais (uniforme, inscrição em torneio).

## Objetivo da v1

1. Emitir uma cobrança Pix identificada por assinatura por mês, e cobranças avulsas quando preciso.
2. Dar baixa automaticamente quando o pagamento cair.
3. Gerar uma lista diária de cobranças a vencer ou vencidas, com mensagem pronta e Pix copia e cola. Uma pessoa envia; nada é enviado automaticamente.

## Decisões

| Data | Decisão | Motivo |
| --- | --- | --- |
| 17/11/2026 | **Só Pix com vencimento (`cobv`)**, sem boleto | Os clientes já pagam por Pix; o boleto exige CPF e endereço completo de quem paga (mais dado pessoal, LGPD); com `cobv` o `txid` é nosso, o que dá idempotência sem truque e faz a baixa independer de quem pagou. Boleto pode entrar depois trocando só o adaptador |
| 17/11/2026 | **Baixa por consulta periódica**, sem webhook na v1 | Dispensa servidor público e mTLS de entrada. O atraso é aceitável para mensalidades; os lembretes trazem a nota "se você já pagou, está tudo certo" e a lista roda uma baixa antes de sair. Webhook fica para depois |
| 17/11/2026 | **Intervalo da baixa configurável por organização** (5 min a 1 dia, padrão 60 min) | Cada negócio tem uma urgência diferente; a rotina só consulta quem tem cobrança aberta |
| 17/11/2026 | **Produto genérico**, sem termos do cliente piloto | Alunos, turmas e chamada ficam no sistema do cliente, ligados por `referencia_externa`. O Em Dia só conhece organizações, assinaturas, cobranças e pagamentos |
| 17/11/2026 | **Sem tabela de pagadores na v1** | A baixa vem pelo `txid`; o contato do lembrete fica na assinatura. Volta se vier boleto |
| 17/11/2026 | **Cobranças recorrentes e avulsas na mesma tabela**, com `chave_idempotencia` única por organização | Baixa, lembretes e relatórios funcionam igual para as duas; pedir duas vezes nunca duplica |
| 17/11/2026 | **Nomes em português** no código e no banco | Produto para o mercado brasileiro |
| 18/11/2026 | **Linguagem e banco de dados ainda a escolher** (roteiro #0); os documentos ficam neutros até lá | A stack precisa permitir mTLS de saída e isolar organizações no próprio banco de dados (por exemplo, Row Level Security do PostgreSQL). Exemplos de tecnologia nos documentos são só exemplos |
| 17/11/2026 | **Adaptador de banco com interface em termos nossos**: emitir, consultar, cancelar, listar pagamentos | Nada do formato do Inter fora do adaptador; outro banco é outro adaptador |
| 17/11/2026 | **Servidor MCP só depois**, começando por consultas | Criar e cancelar com IA no meio pedem confirmação humana |

## Fatos da API do Inter (conferidos em 17/11/2026)

Fonte: [portal do desenvolvedor do Inter](https://developers.inter.co/references/pix).

- **Endereços:** produção `cdpj.partners.bancointer.com.br/pix/v2`, sandbox `cdpj-sandbox.partners.uatinter.co/pix/v2`.
- **Autenticação:** OAuth2 client credentials em `/oauth/v2/token` com certificado do cliente (mTLS). O token vale 60 minutos e só pode ser pedido 5 vezes por minuto, então o cache é obrigatório.
- **Criar:** `PUT /cobv/{txid}`, síncrono. O `txid` é escolhido por nós (26 a 35 letras e números, único por CNPJ). Repetir o PUT numa cobrança ainda `ATIVA` altera a mesma cobrança; numa já paga, dá erro.
- **Corpo:** `calendario.dataDeVencimento`, `calendario.validadeAposVencimento`, `valor.original` (texto, ex.: "125.00"), `chave`, `devedor` (objeto obrigatório com campos opcionais; com nome, exige CPF ou CNPJ), `solicitacaoPagador`.
- **Situações:** `ATIVA`, `CONCLUIDA`, `REMOVIDA_PELO_USUARIO_RECEBEDOR`, `REMOVIDA_PELO_PSP`. Não existe "vencida"; atraso é calculado por nós.
- **Consultar:** `GET /cobv/{txid}`, `GET /cobv` (por período e situação) e `GET /pix` (Pix recebidos, escopo `pix.read`).
- **Escopos necessários:** `cobv.read` e `cobv.write` (e `pix.read`, se usarmos `GET /pix`). Nenhum de pagamento ou transferência.
- **Limites:** 120 chamadas/min em produção e 10/min no sandbox.
- **Webhook (para depois):** um por chave Pix (`PUT /webhook/{chave}`), chamado em `URL/pix`, só para pagamentos, com até 4 novas tentativas (20, 30, 60 e 120 min). O Inter se autentica por certificado.
- **Certificado:** 1 ano em produção, renovável a partir de 90 dias antes do vencimento, mantendo client id e secret. No sandbox vale 30 dias.
- **Sandbox:** conta própria na comunidade do Inter, funciona de segunda a sexta, das 8h às 20h, e tem endpoint para simular pagamento.
- **A conferir no sandbox:** se a resposta traz `pixCopiaECola` (o exemplo da documentação só mostra `loc.location`) e se aceita cobrança sem CPF do devedor.

## Perguntas em aberto

- Dia de vencimento padrão e quantos dias o Pix vale depois de vencer (sugestão: 30).
- Multa, juros ou desconto na v1 (sugestão: nenhum).
- Onde a lista diária aparece: arquivo, página ou mensagem num bot.
- Onde roda em produção e qual cofre de segredos.

Veja também: [processos](processos.md) e [roteiro](roteiro.md).
