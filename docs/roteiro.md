# Roteiro de desenvolvimento

Rascunho de 17/11/2026; vira issues no GitHub. Cada issue é uma entrega pequena, revisada antes da próxima. A ordem segue as dependências.

Rótulos propostos: `etapa-1-base`, `etapa-2-inter`, `etapa-3-operacao`, `etapa-4-producao`, `depois`, `tarefa-manual` (feita por uma pessoa, sem código).

---

## Etapa 1: base, sem banco real

### #1 Esquema do banco: organizações, assinaturas, cobranças e pagamentos
**Objetivo:** criar as quatro tabelas, com isolamento por organização (RLS).
**Inclui:** `migrations/001_inicial.sql` (rascunho em revisão): cobranças recorrentes e avulsas na mesma tabela, `UNIQUE (organizacao_id, chave_idempotencia)` contra duplicata e a função `autenticar_organizacao`.
**Pronto quando:** a migração roda num Postgres vazio sem erro.

### #2 Estrutura do projeto Python e testes automáticos
**Objetivo:** dar ao projeto uma estrutura que roda testes com um comando.
**Inclui:** `pyproject.toml` (uv), pasta `src/emdia`, `pytest`, um Postgres de teste que aplica as migrações, e GitHub Actions rodando os testes a cada push.
**Pronto quando:** `uv run pytest` passa no computador e no GitHub.
**Depende de:** #1

### #3 Identificar a organização pelo token (nunca por parâmetro)
**Objetivo:** o código só descobre a organização a partir de uma credencial.
**Inclui:** `Contexto` criado só por `autenticar(token)`; a conexão grava `app.organizacao_id` para o RLS; um usuário de banco sem superpoderes para o app.
**Pronto quando:** um teste prova que a organização A não lê nem grava nada da organização B, mesmo com `SELECT` sem filtro.
**Depende de:** #2

### #4 txid determinístico
**Objetivo:** cada cobrança ter um identificador Pix calculado, não sorteado.
**Inclui:** função `calcular_txid(organizacao, assinatura, periodo)`, com 26 a 35 letras e números (regra do Banco Central) e sem dado pessoal.
**Pronto quando:** testes mostram o mesmo resultado para a mesma entrada, resultados diferentes para entradas diferentes e o formato sempre válido.
**Depende de:** #2

### #5 Interface do banco e adaptador falso
**Objetivo:** definir o que o Em Dia pede a um banco, em termos nossos, e ter um banco de mentira para testar.
**Inclui:** `AdaptadorBanco` com `emitir_cobranca`, `consultar_cobranca`, `cancelar_cobranca` e `listar_pagamentos`; tipos próprios (sem formato do Inter); `BancoFalso` em memória que imita o Inter: PUT com o mesmo txid não duplica, pode devolver 429 e pode "cair" depois de criar.
**Pronto quando:** testes do `BancoFalso` cobrem esses comportamentos.
**Depende de:** #2

### #6 Job de emissão mensal idempotente
**Objetivo:** gerar uma cobrança por assinatura ativa no período, sem nunca duplicar.
**Inclui:** `python -m emdia emitir --periodo 2026-12`; grava a cobrança primeiro (`a_emitir`) e depois chama o banco; erros do banco viram `falhou` com o motivo.
**Pronto quando:** testes provam uma cobrança por assinatura em três cenários: rodar duas vezes seguidas, queda entre o banco e a gravação, e 429 no meio.
**Depende de:** #3, #4, #5

### #6b Cobranças avulsas (uniforme, torneio)
**Objetivo:** cobrar algo uma vez só, de uma pessoa ou de um grupo.
**Inclui:** `criar_cobranca_avulsa(descricao, valor, vencimento, chave_idempotencia, assinatura?)`; criação em grupo (ex.: torneio de R$ 80 para 25 assinaturas escolhidas), cada uma com a chave `torneio-verao-2026:<referencia>`; contato herdado da assinatura quando houver.
**Pronto quando:** repetir o mesmo pedido devolve a mesma cobrança, e um teste cobre a criação em grupo rodando duas vezes.
**Depende de:** #6

---

## Etapa 2: Inter no sandbox

### #7 Conta e integração no sandbox do Inter `tarefa-manual`
**Objetivo:** ter credenciais de teste.
**Inclui:** criar conta na comunidade do Inter, criar a integração no sandbox e baixar o certificado e a chave (valem 30 dias). Guardar fora do repositório.
**Pronto quando:** temos client id, client secret, certificado e chave de teste guardados num lugar seguro.

### #8 Segredos fora do repositório
**Objetivo:** carregar credenciais sem nunca commitá-las.
**Inclui:** `carregar_segredos(organizacao)` lendo de um cofre local em desenvolvimento; `.gitignore` bloqueando `.crt`, `.key` e `.env`.
**Pronto quando:** o código lê os segredos e um teste garante que nenhum arquivo de segredo pode ser commitado.
**Depende de:** #2

### #9 Adaptador Inter: token, mTLS e limites
**Objetivo:** falar com o Inter de forma segura e educada.
**Inclui:** token OAuth com cache (vale 60 min; só 5 pedidos por minuto), certificado do cliente (mTLS), cabeçalho `x-conta-corrente` quando preciso, recuo exponencial no 429 e no 503. Escopos pedidos: só `cobv.read` e `cobv.write`.
**Pronto quando:** o token é obtido no sandbox e reaproveitado, com testes sobre respostas gravadas.
**Depende de:** #5, #7, #8

### #10 Adaptador Inter: criar, consultar e cancelar Pix com vencimento
**Objetivo:** implementar a interface para valer.
**Inclui:** `PUT/GET/PATCH /pix/v2/cobv/{txid}`; tradução de situações e de valores (texto para centavos). Responder duas dúvidas no sandbox: o Inter devolve o copia e cola? Aceita cobrança sem CPF do devedor?
**Pronto quando:** os testes rodam sobre respostas reais gravadas do sandbox, incluindo erros 400, 429 e 503.
**Depende de:** #9

### #11 Emissão mensal de ponta a ponta no sandbox
**Objetivo:** ver o job #6 funcionando com o Inter de verdade.
**Inclui:** dados inventados (nunca pessoas reais); ritmo de até 8 chamadas por minuto no sandbox.
**Pronto quando:** rodar duas vezes cria exatamente uma cobrança por assinatura no sandbox.
**Depende de:** #6, #10

### #12 Baixa automática com intervalo configurável
**Objetivo:** marcar cobranças como pagas sem ninguém conferir à mão.
**Inclui:** `listar_pagamentos` no adaptador Inter; `python -m emdia baixar`, que roda a cada 5 minutos mas só consulta o banco das organizações cujo `intervalo_baixa_min` (5 min a 1 dia, padrão 60) já passou desde `ultima_baixa_em`; organizações sem cobrança aberta não consultam; janela de sobra de 1 hora e varredura diária dos últimos 7 dias; pagamento com txid desconhecido ou valor diferente vai para o relatório, sem baixa.
**Pronto quando:** um pagamento simulado no sandbox vira `paga` e o mesmo pagamento visto duas vezes não duplica.
**Depende de:** #10

---

## Etapa 3: operação

### #13 Lista diária de lembretes
**Objetivo:** uma pessoa saber, todo dia, quem lembrar e o que mandar.
**Inclui:** cobranças abertas que vencem em até 3 dias ou já venceram; agrupadas por telefone de contato; mensagem de modelo fixo (sem IA) com o copia e cola e a nota "Se você já pagou, está tudo certo; a confirmação pode levar alguns minutos para aparecer aqui."; uma baixa roda antes de montar a lista. Nada é enviado automaticamente.
**Em aberto:** onde a lista aparece (arquivo, página ou Telegram).
**Depende de:** #12

### #14 Alertas: certificado, falhas e pagamentos sem dono
**Objetivo:** não ser pego de surpresa.
**Inclui:** aviso do vencimento do certificado a partir de 90 dias, depois em 30, 14 e 7 dias, e diário na última semana; cobranças `falhou`; pagamentos sem cobrança. Tudo no topo da lista diária.
**Depende de:** #13

### #15 Importar assinaturas de uma planilha
**Objetivo:** cadastrar as assinaturas sem digitar uma a uma.
**Inclui:** importar CSV/XLSX com descrição, referência externa, valor, dia de vencimento e contato; reimportar atualiza em vez de duplicar (pela referência externa). Testes só com dados fictícios.
**Depende de:** #3

### #16 Hospedagem e agendamento
**Objetivo:** rodar sozinho, todo dia, num lugar seguro.
**Inclui:** escolher onde roda (VPS ou serviço de jobs), Postgres, cofre de segredos de produção, agendamento de emitir (dia 1), baixar (a cada 5 min, consultando só as organizações cujo intervalo já passou) e lista (todo dia de manhã), e backup do banco.
**Depende de:** #12, #13

---

## Etapa 4: produção

### #17 Integração de produção do cliente piloto `tarefa-manual`
**Objetivo:** credenciais reais da conta PJ do cliente piloto, criadas pelo responsável pela conta.
**Inclui:** no Internet Banking do Inter, em Integrar → Minhas integrações: criar a integração com os escopos `cobv.read` e `cobv.write` (e `pix.read`, se a #12 usar `GET /pix`), baixar o certificado e a chave, e confirmar qual chave Pix do CNPJ usar.

### #18 Piloto com uma turma
**Objetivo:** testar com gente de verdade, em pequena escala.
**Inclui:** uma turma por um mês; comparar com a conferência manual; ajustar as mensagens.
**Depende de:** #16, #17

---

## Depois (fora da v1)

- **Webhook do Pix** para baixa em segundos (exige endereço HTTPS público e mTLS de entrada).
- **Servidor MCP** com `consultar_cobranca` e `listar_atrasadas` primeiro; criar e cancelar só com confirmação.
- **Pix Automático** para quem topar o débito recorrente.
- **Boleto**, se aparecer demanda (aí volta a tabela de pagadores, com CPF e endereço).
- **Recebimentos em conta de pessoa física**: fora, porque a API é só para conta PJ.
