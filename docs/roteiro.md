# Roteiro de desenvolvimento

Backlog, não plano fixo. Escrito em 17/11/2026, antes de qualquer contato com o banco real, então é uma aposta. O trabalho anda em [iterações](iteracoes.md) de ponta a ponta, e este roteiro é revisado depois de cada uma: itens mudam, somem ou trocam de ordem. Cada item é uma entrega pequena; a ordem abaixo segue as dependências, não a ordem em que serão feitos.

Os itens são numerados por etapa (E1.3 é o terceiro item da etapa 1) para não se confundirem com os números das issues do GitHub. Quando um item depende de uma issue, ela aparece como link.

Rótulos propostos: `etapa-0-decisoes`, `etapa-1-base`, `etapa-2-inter`, `etapa-3-operacao`, `etapa-4-producao`, `depois`, `tarefa-manual` (feita por uma pessoa, sem código).

Issues de decisão já abertas (perguntas da [iteração 1](iteracoes.md#iteração-1-uma-cobrança-no-sandbox-do-inter)):

- [#1 Dados do devedor na cobv](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/1)
- [#2 PUT ou PATCH na cobv para garantir idempotência](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/2)
- [#3 Conferir os números e detalhes da API do Inter](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/3)

---

## Etapa 0: decisões antes do código

### E0.1 Escolher a stack ✓
**Feito em 18/11/2026:** Rust + PostgreSQL (`sqlx`, `tokio`, `reqwest`); motivo nas decisões de [contexto.md](contexto.md).
**Objetivo:** decidir a linguagem e o banco de dados antes de escrever código.
**Inclui:** comparar as opções (por exemplo Python, TypeScript ou Go) pelo que o produto exige: cliente HTTPS com certificado (mTLS) e cache de token OAuth, rotinas agendadas (emitir, baixar, lista do dia), dinheiro em centavos inteiros, testes de idempotência simples de escrever e código fácil de revisar para quem está aprendendo. No banco de dados: isolamento por organização garantido pelo próprio banco (por exemplo, Row Level Security no Postgres) e restrições `UNIQUE` contra duplicata.
**Pronto quando:** a escolha e o motivo estão registrados nas decisões de [contexto.md](contexto.md), e o README e o AGENTS.md trazem os comandos de instalar e testar.

### E0.2 Escolher a interface ✓
**Feito em 18/11/2026:** no piloto, a interface é a **planilha Google que a equipe já usa**, com abas para assinaturas, cobranças avulsas e vínculos de Pix sem cobrança. O Em Dia lê a planilha numa rotina, mantém o próprio registro e só escreve nas colunas de situação. Depois do protótipo, uma interface com autenticação própria pode substituir a planilha. Motivo nas decisões de [contexto.md](contexto.md).

---

## Etapa 1: base, sem banco real

### E1.2 Estrutura do projeto e testes automáticos ✓
**Feito em 18/11/2026:** projeto `cargo` sem dependências e GitHub Actions rodando `cargo fmt --check`, `cargo clippy` e `cargo test`. As dependências entram com o primeiro código que as usa (`sqlx` e `tokio` no E1.1, `reqwest` no adaptador do Inter).
**Objetivo:** dar ao projeto uma estrutura que roda testes com um comando.
**Inclui:** o mínimo para compilar e testar: `Cargo.toml`, `src/lib.rs` e o workflow de CI. Sem banco ainda; o Postgres no CI vem com a primeira migração (E1.1).
**Pronto quando:** `cargo test` passa no computador e no GitHub.
**Depende de:** E0.1

### E1.1 Esquema do banco: organizações, contatos, assinaturas, cobranças e pagamentos
**Objetivo:** criar as cinco tabelas, com isolamento por organização garantido pelo Row Level Security do PostgreSQL.
**Inclui:** a primeira migração: cobranças recorrentes e avulsas na mesma tabela, com `assinatura_id` (opcional na avulsa) e `motivo_falha`; `contatos` com nome e telefone, apontado por assinaturas e cobranças; `pagamentos.cobranca_id` opcional, para guardar pagamentos sem dono; `UNIQUE (organizacao_id, chave_idempotencia)` contra duplicata; e a função `autenticar_organizacao`. O modelo completo está em [processos.md](processos.md#modelo-de-dados). Traz junto o que só faz sentido com uma migração: `sqlx` e `tokio` no `Cargo.toml`, migrações em `migrations/` aplicadas pelo `sqlx`, testes com `sqlx::test` (um banco novo por teste, com as migrações aplicadas), cache offline do `sqlx` (`.sqlx/`) commitado e um Postgres no GitHub Actions.
**Pronto quando:** a migração roda num banco vazio sem erro, e `cargo test` passa com ela no computador e no GitHub.
**Depende de:** E1.2

### E1.3 Identificar a organização pelo token (nunca por parâmetro)
**Objetivo:** o código só descobre a organização a partir de uma credencial.
**Inclui:** `Contexto` criado só por `autenticar(token)` (para uma futura página web ou API) ou pela rotina agendada, que percorre as organizações uma a uma; a organização é informada ao banco **a cada transação** (`set_config('app.organizacao_id', ..., true)`, equivalente a `SET LOCAL`), lida pelas políticas de RLS, para funcionar com o pool de conexões do Neon; um usuário de banco sem superpoderes para o app.
**Pronto quando:** um teste prova que a organização A não lê nem grava nada da organização B, mesmo numa consulta sem filtro de organização.
**Depende de:** E1.2

### E1.4 Criar uma organização e emitir o token dela
**Objetivo:** ter um jeito seguro de cadastrar uma organização nova.
**Inclui:** comando de administração que cria a organização (nome, chave Pix, dias após vencimento), gera um token aleatório, mostra o token uma única vez e grava só o hash dele. Também permite trocar o token, invalidando o anterior.
**Pronto quando:** testes mostram que o token gerado autentica a organização certa, que o token não fica gravado em texto puro e que, depois da troca, o token antigo não funciona mais.
**Depende de:** E1.3

### E1.5 txid determinístico ✓
**Feito em 20/11/2026:** `calcular_txid` em `src/txid.rs`; motivo nas decisões de [contexto.md](contexto.md).
**Objetivo:** cada cobrança ter um identificador Pix calculado, não sorteado.
**Inclui:** função `calcular_txid(organizacao, chave_idempotencia)`, um hash com 26 a 35 letras e números (regra do Banco Central) e sem dado pessoal. Na recorrente a chave é `mensal:<assinatura>:<periodo>`; na avulsa, a que o pedido informa. A chave decide o que é duplicata e o `txid` sai dela, então os dois nunca discordam.
**Pronto quando:** testes mostram o mesmo resultado para a mesma entrada, resultados diferentes para entradas diferentes e o formato sempre válido.
**Depende de:** E1.2

### E1.6 Interface do banco e adaptador falso
**Objetivo:** definir o que o Em Dia pede a um banco, em termos nossos, e ter um banco de mentira para testar.
**Inclui:** `AdaptadorBanco` com `emitir_cobranca`, `consultar_cobranca`, `cancelar_cobranca` e `listar_pagamentos`; tipos próprios (sem formato do Inter); `BancoFalso` em memória que imita o Inter: emitir de novo com o mesmo txid não duplica, pode devolver 429 e pode "cair" depois de criar.
**Pronto quando:** testes do `BancoFalso` cobrem esses comportamentos.
**Depende de:** E1.2

### E1.7 Rotina de emissão mensal idempotente
**Objetivo:** gerar uma cobrança por assinatura ativa no período, sem nunca duplicar.
**Inclui:** comando `emitir --periodo 2026-12`; grava a cobrança primeiro (`a_emitir`) e depois chama o banco; erros do banco viram `falhou` com o motivo em `motivo_falha`. Processa em lotes com limite de tempo: cada execução para antes de 15 minutos (limite do Lambda) e a próxima continua das que ficaram em `a_emitir`.
**Pronto quando:** testes provam uma cobrança por assinatura em quatro cenários: rodar duas vezes seguidas, queda entre o banco e a gravação, 429 no meio, e execução interrompida pelo limite de tempo e retomada.
**Depende de:** E1.3, E1.5, E1.6

### E1.8 Cobranças avulsas
**Objetivo:** cobrar algo uma vez só, de uma pessoa ou de um grupo.
**Inclui:** `criar_cobranca_avulsa(descricao, valor, vencimento, chave_idempotencia, assinatura?)`; criação em grupo (ex.: uma taxa de evento de R$ 80 para 25 assinaturas escolhidas), cada uma com a chave `evento-2026:<referencia>`; contato herdado da assinatura quando houver, ou informado no pedido para quem não tem assinatura.
**Pronto quando:** repetir o mesmo pedido devolve a mesma cobrança, e um teste cobre a criação em grupo rodando duas vezes.
**Depende de:** E1.7

---

## Etapa 2: Inter no sandbox

### E2.1 Conta e integração no sandbox do Inter `tarefa-manual`
**Objetivo:** ter credenciais de teste.
**Inclui:** criar conta na comunidade do Inter, criar a integração no sandbox e baixar o certificado e a chave (valem 30 dias). Guardar fora do repositório.
**Pronto quando:** temos client id, client secret, certificado e chave de teste guardados num lugar seguro.

### E2.2 Segredos fora do repositório
**Objetivo:** carregar credenciais sem nunca commitá-las.
**Inclui:** `carregar_segredos(organizacao)` lendo de um cofre local em desenvolvimento; `.gitignore` bloqueando `.crt`, `.key` e `.env`.
**Pronto quando:** o código lê os segredos e um teste garante que nenhum arquivo de segredo pode ser commitado.
**Depende de:** E1.2

### E2.3 Adaptador Inter: token, mTLS e limites
**Objetivo:** falar com o Inter de forma segura e educada.
**Inclui:** token OAuth com cache (vale 60 min; só 5 pedidos por minuto), certificado do cliente (mTLS), cabeçalho `x-conta-corrente` quando preciso, recuo exponencial no 429 e no 503. Escopos pedidos: `cobv.read`, `cobv.write` e `pix.read`. O `pix.read` é temporário: sai depois de cerca de dois meses sem nenhum Pix recebido fora de uma cobrança.
**Pronto quando:** o token é obtido no sandbox e reaproveitado, com testes sobre respostas gravadas.
**Depende de:** E1.6, E2.1, E2.2, [issue #3](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/3) (números do Inter)

### E2.4 Adaptador Inter: criar, consultar e cancelar Pix com vencimento
**Objetivo:** implementar a interface para valer.
**Inclui:** as chamadas de `/pix/v2/cobv/{txid}` para criar, consultar e cancelar, do jeito decidido na issue #2; os dados do devedor como decidido na issue #1; tradução de situações e de valores (texto para centavos); conferir no sandbox se a resposta traz o Pix copia e cola.
**Pronto quando:** os testes rodam sobre respostas reais gravadas do sandbox, incluindo erros 400, 429 e 503.
**Depende de:** E2.3, [issue #1](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/1) (devedor), [issue #2](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/2) (PUT ou PATCH), [issue #3](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/3) (números do Inter)

### E2.5 Emissão mensal de ponta a ponta no sandbox
**Objetivo:** ver a rotina da E1.7 funcionando com o Inter de verdade.
**Inclui:** dados inventados (nunca pessoas reais); ritmo de até 8 chamadas por minuto no sandbox.
**Pronto quando:** rodar duas vezes cria exatamente uma cobrança por assinatura no sandbox.
**Depende de:** E1.7, E2.4

### E2.6 Baixa automática diária
**Objetivo:** marcar cobranças como pagas sem ninguém conferir à mão.
**Inclui:** `listar_pagamentos` no adaptador Inter; comando `baixar`, que roda toda manhã antes da lista do dia e consulta o banco das organizações com cobrança aberta, desde a `ultima_baixa_em` de cada uma; organizações sem cobrança aberta não consultam; janela de sobra de 1 hora e varredura diária dos últimos 7 dias; pagamento com txid desconhecido ou valor diferente é gravado em `pagamentos` com `cobranca_id` vazio, sem baixa, para o relatório e os alertas; cobranças abertas cujo vencimento mais `dias_apos_vencimento` já passou viram `expirada`.
**Pronto quando:** um pagamento simulado no sandbox vira `paga`, o mesmo pagamento visto duas vezes não duplica, e um teste mostra a cobrança vencida há mais que o prazo virando `expirada`.
**Depende de:** E2.4

---

## Etapa 3: operação

### E3.1 Lista diária de lembretes
**Objetivo:** uma pessoa saber, todo dia, quem lembrar e o que mandar.
**Inclui:** cobranças abertas que vencem em até 3 dias ou já venceram; agrupadas por telefone de contato; mensagem de modelo fixo (sem IA) com o copia e cola e a nota "Se você já pagou, está tudo certo; a confirmação pode levar alguns minutos para aparecer aqui."; uma baixa roda antes de montar a lista. Nada é enviado automaticamente.
**Onde aparece:** a definir entre uma aba da planilha (E3.4) escrita pelo Em Dia ou outro canal; veja as perguntas em aberto de [contexto.md](contexto.md).
**Depende de:** E2.6, [issue #3](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/3) (se a resposta da cobv traz `pixCopiaECola`)

### E3.2 Alertas: certificado, falhas e pagamentos sem dono
**Objetivo:** não ser pego de surpresa.
**Inclui:** aviso do vencimento do certificado a partir de 90 dias, depois em 30, 14 e 7 dias, e diário na última semana; cobranças `falhou`; pagamentos sem cobrança (`cobranca_id` vazio). Tudo no topo da lista diária.
**Depende de:** E3.1

### E3.3 Cancelar uma cobrança
**Objetivo:** desistir de uma cobrança sem deixar nada pela metade.
**Inclui:** `cancelar_cobranca(cobranca)`: em `a_emitir` ou `falhou`, só muda para `cancelada` localmente; em `aberta`, chama `cancelar_cobranca` no banco e então marca `cancelada`. Paga ou expirada não pode ser cancelada.
**Pronto quando:** teste de idempotência: cancelar duas vezes dá o mesmo resultado, e uma queda entre o banco e a gravação se completa na próxima tentativa sem erro.
**Depende de:** E2.4

### E3.4 Conexão com a planilha da organização
**Objetivo:** o Em Dia ler e escrever na planilha Google de cada organização com segurança.
**Inclui:** Google Sheets API com uma conta de serviço num projeto Google Cloud próprio; cada organização compartilha a planilha com o e-mail da conta de serviço como editora (ela não vê mais nada do Drive); a chave JSON no Parameter Store (`SecureString`), legível só pelo papel do Lambda, trocada uma vez por ano junto com o certificado do Inter; token de acesso de 1 hora com o escopo `spreadsheets`; o id da planilha guardado por organização; modelo de planilha com as abas Assinaturas, Avulsas e Vínculos, cada linha com uma coluna `id` estável (não o número da linha) e colunas de situação que só o Em Dia escreve, protegidas contra edição da equipe. Testes com uma planilha falsa em memória, como o `BancoFalso`.
**Pronto quando:** a rotina lê as três abas de uma planilha de teste, escreve uma situação e não altera nenhuma outra coluna.
**Depende de:** E0.2, E1.3, E2.2

### E3.5 Hospedagem e agendamento
**Objetivo:** rodar sozinho, todo dia, num lugar seguro.
**Inclui:** infraestrutura como código em `sa-east-1`: funções Lambda (Rust com `cargo lambda`, fora de VPC), agendas no EventBridge Scheduler para emitir (dia 1), baixar e lista (todo dia de manhã, nessa ordem), ler a planilha (de hora em hora, das 8h às 20h), Parameter Store para os segredos, papel IAM mínimo, banco no Neon (São Paulo) e alertas de falha das funções. Acompanhar o consumo de computação do Neon (a baixa diária e as leituras da planilha devem usar umas 8 das 100 horas gratuitas por mês); desenvolvimento e testes usam outro projeto no Neon, com cota própria. Conferir a retenção de histórico do Neon como backup.
**Pronto quando:** um `deploy` sobe tudo do zero e as rotinas rodam no horário no sandbox.
**Depende de:** E2.6, E3.1

### E3.6 Ler as assinaturas da planilha
**Objetivo:** a planilha continua sendo a lista mestra de assinaturas, sem ninguém mudar o hábito.
**Inclui:** rotina agendada que lê a aba Assinaturas (descrição, referência externa, valor, dia de vencimento, contato, ativa) e atualiza o registro do Em Dia pela referência externa, sem duplicar; cada linha recebe na coluna de situação "ok" ou o problema encontrado (valor inválido, referência repetida, telefone vazio), em vez de falhar em silêncio. Linha apagada da planilha não apaga nada: a assinatura fica inativa e as cobranças continuam no histórico. Testes só com dados fictícios.
**Pronto quando:** ler a mesma planilha duas vezes não muda nada, uma linha com erro aparece marcada e não para as outras, e uma alteração de valor vale a partir da próxima emissão.
**Depende de:** E3.4

### E3.7 Dar baixa à mão pela aba Vínculos
**Objetivo:** ligar um Pix sem cobrança (ou um pagamento em dinheiro) à cobrança certa, para quem já pagou sair da lista.
**Inclui:** o Em Dia escreve na aba Vínculos cada Pix sem cobrança (nome de quem pagou, valor, hora); a equipe preenche a referência da cobrança, ou cria uma linha de pagamento em dinheiro; a rotina lê a aba, grava o vínculo, marca a cobrança como `paga` e escreve o resultado na coluna de situação. Pagamento em dinheiro cria um pagamento sem `pix_e2e_id`, com origem "manual". Apagar o vínculo na planilha desfaz a baixa à mão e volta a cobrança para `aberta`.
**Pronto quando:** testes de idempotência (ler a mesma linha duas vezes não duplica nem muda nada), não deixa ligar um pagamento a uma cobrança já paga (a linha fica marcada com o motivo), e a cobrança some da próxima lista.
**Depende de:** E2.6, E3.4

### E3.8 Criar cobrança avulsa pela aba Avulsas
**Objetivo:** criar uma cobrança única, para uma assinatura ou um grupo, sem precisar de quem programa.
**Inclui:** a cada leitura da planilha, cada linha nova da aba Avulsas (descrição, valor, vencimento, referências das assinaturas) vira cobrança por `criar_cobranca_avulsa` (E1.8), com a chave de idempotência `planilha:<id da linha>:<referência>`; a coluna de situação mostra "emitida", "falhou" e o motivo, ou o Pix copia e cola. Editar uma linha já emitida não muda a cobrança: a coluna de situação avisa e a correção é cancelar (E3.3) e criar outra linha. Venda no balcão, paga na hora, não passa pela aba Avulsas: fica fora do Em Dia.
**Pronto quando:** ler a mesma linha duas vezes cria uma cobrança só por assinatura, emitida no banco como as outras.
**Depende de:** E1.8, E2.4, E3.4

---

## Etapa 4: produção

### E4.1 Integração de produção do cliente piloto `tarefa-manual`
**Objetivo:** credenciais reais da conta PJ do cliente piloto, criadas pelo responsável pela conta.
**Inclui:** no Internet Banking do Inter, em Integrar → Minhas integrações: criar a integração com os escopos `cobv.read`, `cobv.write` e `pix.read` (temporário, como na E2.3), baixar o certificado e a chave, e confirmar qual chave Pix do CNPJ usar.

### E4.2 Piloto em pequena escala
**Objetivo:** testar com gente de verdade, em pequena escala.
**Inclui:** um grupo pequeno de assinaturas por um mês; comparar com a conferência manual; ajustar as mensagens.
**Depende de:** E3.5, E4.1

---

## Depois (fora da v1)

- **Botão "Gerar Pix" na planilha**, se a venda no balcão passar a ser cobrança do Em Dia: um Apps Script chama uma Function URL do Lambda só com "processe a organização X agora" (sem dados, com segredo e limite de chamadas); o Em Dia lê a planilha pela conta de serviço, emite a cobrança vencendo no dia e escreve o copia e cola na linha em segundos.
- **Interface própria no lugar da planilha**, com autenticação própria, quando os fluxos da planilha estiverem provados no protótipo.
- **Intervalo da baixa por organização**, com uma agenda do EventBridge por organização, se algum cliente precisar.
- **Webhook do Pix** para baixa em segundos (exige endereço HTTPS público e mTLS de entrada).
- **Servidor MCP** com `consultar_cobranca` e `listar_atrasadas` primeiro; criar e cancelar só com confirmação.
- **Pix Automático** para quem topar o débito recorrente.
- **Boleto**, se aparecer demanda (CPF e endereço entram em `contatos`).
- **Juntar contatos repetidos** (a mesma pessoa em várias linhas da planilha), quando a interface própria deixar a equipe dizer quem é quem.
- **Criptografar nome e telefone** de `contatos` no próprio Em Dia, com chave no Parameter Store e um HMAC do telefone para agrupar a lista, se o risco justificar. Até lá vale a criptografia em repouso do Neon e o TLS.
- **Recebimentos em conta de pessoa física**: fora, porque a API é só para conta PJ.
