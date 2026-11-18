# AGENTS.md

Instruções para agentes de código (Claude, Codex, Copilot etc.) que trabalham neste repositório. Para o que o projeto é, veja o README.

## Comandos

A definir junto com a escolha da linguagem e do banco de dados. Ainda não há código.

## Regras que nunca se quebram

1. **Nunca duplicar cobrança.** Toda escrita é idempotente. Toda cobrança nova tem `chave_idempotencia`; recorrentes a calculam a partir de assinatura e período. Não remova nem afrouxe os `UNIQUE` de `cobrancas` e `pagamentos`.
2. **Organização vem do contexto autenticado**, nunca de um argumento. Funções de serviço recebem um `Contexto`, não um `organizacao_id`. Não use um usuário de banco que ignore o isolamento por organização (por exemplo, o RLS do PostgreSQL).
3. **Nada do formato do banco fora do adaptador.** JSON, nomes de campo e situações do Inter ficam só no adaptador do Inter. O resto do código usa só os tipos do domínio.
4. **Sem IA no caminho do dinheiro.** Emissão, baixa e cancelamento são código determinístico.
5. **Segredos nunca no repositório.** Nada de `.crt`, `.key`, `.env`, tokens ou client secrets em código, testes ou fixtures. Respostas gravadas do sandbox passam por limpeza antes de entrar em `tests/`.
6. **Permissão mínima.** Só os escopos `cobv.read` e `cobv.write` (e `pix.read`, se necessário). Nunca escopos de pagamento ou transferência.
7. **Nunca dados reais de pessoas.** Testes e exemplos usam dados inventados.

## Convenções

- **Português** nos nomes de tabelas, colunas, funções e variáveis.
- **Dinheiro em centavos inteiros** (`valor_centavos`). Nunca ponto flutuante. Conversão para o formato do banco ("125.00") só no adaptador.
- **Datas:** data sem hora para vencimento e período (sempre o dia 1 do mês); data e hora com fuso para eventos.
- **Migrações** em `migrations/NNN_descricao`, só para frente. Não edite uma migração que já foi para a `main`.
- **Adaptador falso** (`BancoFalso`) imita o comportamento do Inter, inclusive erros, 429 e quedas. Mudou o adaptador real, atualize o falso.

## Testes

- Toda mudança em emissão ou baixa precisa de teste de idempotência: rodar duas vezes, falha no meio, mesmo pagamento visto duas vezes.
- O adaptador Inter é testado contra respostas gravadas do sandbox, sem rede.
- Testes de isolamento: a organização A não lê nem escreve dados da B.

## Forma de trabalhar

- Uma issue por pull request, pequena o bastante para ser lida de uma vez.
- Explique no PR o porquê das decisões; o dono do projeto revisa para aprender.
- Na dúvida entre a issue e a documentação do banco, pare e pergunte em vez de adivinhar.
