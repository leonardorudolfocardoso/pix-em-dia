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
| 17/11/2026 | ~~Intervalo da baixa configurável por organização (5 min a 1 dia, padrão 60 min)~~ | Substituída em 18/11/2026 pela linha abaixo |
| 18/11/2026 | **Baixa em intervalo fixo de 15 minutos para todas as organizações**; intervalo por organização fica para depois | Custo: cada execução consulta o banco, e o Neon só dorme depois de 5 minutos sem consultas. Acordando a cada 5 minutos o banco nunca dorme (cerca de 180 horas de computação por mês, acima das 100 gratuitas, uns US$ 9/mês). A cada 15 minutos são cerca de 60 horas, dentro da camada gratuita com folga (contando o menor tamanho de computação, 0,25 CU); a 10 minutos seriam uns 92, sem folga. Também simplifica: uma agenda só no EventBridge e nenhum campo de intervalo por organização. Se um cliente precisar de outro intervalo, a volta é uma agenda por organização |
| 17/11/2026 | **Produto genérico**, sem termos do cliente piloto | Alunos, turmas e chamada ficam no sistema do cliente, ligados por `referencia_externa`. O Em Dia só conhece organizações, assinaturas, cobranças e pagamentos |
| 17/11/2026 | **Sem tabela de pagadores na v1** | A baixa vem pelo `txid`; o contato do lembrete fica na assinatura. Volta se vier boleto |
| 17/11/2026 | **Cobranças recorrentes e avulsas na mesma tabela**, com `chave_idempotencia` única por organização | Baixa, lembretes e relatórios funcionam igual para as duas; pedir duas vezes nunca duplica |
| 17/11/2026 | **Nomes em português** no código e no banco | Produto para o mercado brasileiro |
| 18/11/2026 | **Rust + PostgreSQL** (`sqlx`, `tokio`, `reqwest`), roteiro E0.1 | O dono do projeto já conhece Rust. O compilador guarda as regras do dinheiro: `Centavos` como tipo próprio (nunca misturado com o texto "125.00" do banco) e situações da cobrança como `enum`, com todo `match` obrigado a tratar todas. O `sqlx` confere cada consulta contra o esquema real na compilação e aplica as migrações em SQL puro, só para frente, recusando migração já aplicada que foi editada. PostgreSQL isola organizações no próprio banco com Row Level Security e garante os `UNIQUE` contra duplicata. mTLS de saída com `reqwest` e certificado em PEM. Um binário só, simples de agendar. Custos aceitos: começo mais lento que em Python, mais cerimônia no adaptador assíncrono e no `BancoFalso`, e `cargo sqlx prepare` depois de cada migração. Esquema, RLS e migrações são SQL e sobrevivem a uma troca de linguagem |
| 18/11/2026 | **PostgreSQL gerenciado no Neon, região São Paulo** | Custo perto de zero no volume da v1 (camada gratuita, o banco dorme quando ocioso), acesso por TLS sem rede privada nem NAT, dados no Brasil e PostgreSQL comum, com RLS. Na mesma comparação, RDS custaria cerca de US$ 28 a 30/mês em São Paulo e exigiria NAT ou endpoint público para as funções alcançarem o Inter; Aurora DSQL não tem RLS. Como é PostgreSQL comum, mudar para RDS depois é `pg_dump`/restauração. Neon fica registrado como operador de dados para a LGPD |
| 18/11/2026 | **Rotinas em AWS Lambda, agendadas pelo EventBridge Scheduler, na região São Paulo** (`sa-east-1`), com infraestrutura como código | Custo perto de zero (a camada sempre gratuita do Lambda cobre milhares de execuções curtas por mês), nenhum servidor para manter, binário Rust nativo (`cargo lambda`) e dados no Brasil ao lado do Neon. Lambda fora de VPC: alcança o Inter e o Neon direto, sem NAT. Certificado, chave e client secret no Parameter Store (`SecureString`). Comparado: VPS em São Paulo (cerca de US$ 5 a 7,50/mês, com sistema operacional para manter) e Cloudflare Containers (cerca de US$ 5/mês, produto mais novo) |
| 17/11/2026 | **Adaptador de banco com interface em termos nossos**: emitir, consultar, cancelar, listar pagamentos | Nada do formato do Inter fora do adaptador; outro banco é outro adaptador |
| 18/11/2026 | **Escopos `cobv.read` e `cobv.write`, mais `pix.read` com prazo** | Emitir e cancelar usam `cobv.write`; consultar e a baixa usam `cobv.read` (`GET /cobv` concluídas). Na transição, quem paga ainda manda Pix direto na chave, sem `txid`; sem `pix.read` o Em Dia lembraria quem já pagou. Com ele, a baixa lê `GET /pix`, guarda só os Pix sem `txid` e os mostra na lista do dia para uma pessoa ligar à cobrança certa. Depois de uns dois meses sem Pix sem cobrança, o `pix.read` sai da integração no Internet Banking, sem mudar código |
| 17/11/2026 | **Servidor MCP só depois**, começando por consultas | Criar e cancelar com IA no meio pedem confirmação humana |

## Fatos da API do Inter (conferidos em 17/11/2026)

Fonte: [portal do desenvolvedor do Inter](https://developers.inter.co/references/pix).

- **Endereços:** produção `cdpj.partners.bancointer.com.br/pix/v2`, sandbox `cdpj-sandbox.partners.uatinter.co/pix/v2`.
- **Autenticação:** OAuth2 client credentials em `/oauth/v2/token` com certificado do cliente (mTLS). O token vale 60 minutos e só pode ser pedido 5 vezes por minuto, então o cache é obrigatório.
- **Criar:** `PUT /cobv/{txid}`, síncrono. O `txid` é escolhido por nós (26 a 35 letras e números, único por CNPJ). Repetir o PUT numa cobrança ainda `ATIVA` altera a mesma cobrança; numa já paga, dá erro.
- **Corpo:** `calendario.dataDeVencimento`, `calendario.validadeAposVencimento`, `valor.original` (texto, ex.: "125.00"), `chave`, `devedor` (objeto obrigatório com campos opcionais; com nome, exige CPF ou CNPJ), `solicitacaoPagador`.
- **Situações:** `ATIVA`, `CONCLUIDA`, `REMOVIDA_PELO_USUARIO_RECEBEDOR`, `REMOVIDA_PELO_PSP`. Não existe "vencida"; atraso é calculado por nós.
- **Consultar:** `GET /cobv/{txid}`, `GET /cobv` (por período e situação) e `GET /pix` (Pix recebidos, escopo `pix.read`).
- **Escopos:** `cobv.write` para criar (`PUT`) e cancelar (`PATCH /cobv/{txid}`); `cobv.read` para consultar; `pix.read` para `GET /pix`; `webhook.read` e `webhook.write` só quando vier o webhook. `pix.write` (devolução) e escopos de pagamento ou transferência nunca.
- **Limites:** 120 chamadas/min em produção e 10/min no sandbox.
- **Webhook (para depois):** um por chave Pix (`PUT /webhook/{chave}`), chamado em `URL/pix`, só para pagamentos, com até 4 novas tentativas (20, 30, 60 e 120 min). O Inter se autentica por certificado.
- **Certificado:** 1 ano em produção, renovável a partir de 90 dias antes do vencimento, mantendo client id e secret. No sandbox vale 30 dias.
- **Sandbox:** conta própria na comunidade do Inter, funciona de segunda a sexta, das 8h às 20h, e tem endpoint para simular pagamento.
- **A conferir no sandbox:** se a resposta traz `pixCopiaECola` (o exemplo da documentação só mostra `loc.location`) e se aceita cobrança sem CPF do devedor; se o período de `GET /cobv` filtra pela data de criação (como no padrão do Banco Central) ou de pagamento, e se a resposta traz `pix[].endToEndId`; se `GET /pix` informa o nome de quem pagou.

## Perguntas em aberto

- Dia de vencimento padrão e quantos dias o Pix vale depois de vencer (sugestão: 30).
- Multa, juros ou desconto na v1 (sugestão: nenhum).
- Onde a lista diária aparece: arquivo, página ou mensagem num bot.
- Qual ferramenta de infraestrutura como código (SAM, CDK ou Terraform).

Veja também: [processos](processos.md) e [roteiro](roteiro.md).
