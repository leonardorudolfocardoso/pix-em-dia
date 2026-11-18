# AGENTS.md

Instruções para agentes de código (Claude, Codex, Copilot etc.) que trabalham neste repositório. Para o que o projeto é, veja o README.

## Comandos

Rust + PostgreSQL, com `sqlx`, `tokio` e `reqwest`. Ainda não há código; estes são os comandos previstos a partir do item E1.2 do roteiro.

```bash
cargo build                       # compilar
sqlx migrate add descricao        # nova migração em migrations/
sqlx migrate run                  # aplicar migrações (usa DATABASE_URL)
cargo test                        # todos os testes (precisa de Postgres local)
cargo test nome                   # um teste
cargo sqlx prepare                # atualizar o cache offline do sqlx depois de mudar consultas ou migrações
cargo clippy -- -D warnings       # lint
cargo fmt                         # formatação
```

## Regras que nunca se quebram

1. **Nunca duplicar cobrança.** Toda escrita é idempotente. Toda cobrança nova tem `chave_idempotencia`; recorrentes a calculam a partir de assinatura e período. Não remova nem afrouxe os `UNIQUE` de `cobrancas` e `pagamentos`.
2. **Organização vem do contexto autenticado**, nunca de um argumento. Funções de serviço recebem um `Contexto`, não um `organizacao_id`. Não use um usuário de banco que ignore o Row Level Security do PostgreSQL.
3. **Nada do formato do banco fora do adaptador.** JSON, nomes de campo e situações do Inter ficam só no adaptador do Inter. O resto do código usa só os tipos do domínio.
4. **Sem IA no caminho do dinheiro.** Emissão, baixa e cancelamento são código determinístico.
5. **Segredos nunca no repositório.** Nada de `.crt`, `.key`, `.env`, tokens ou client secrets em código, testes ou fixtures. Respostas gravadas do sandbox passam por limpeza antes de entrar em `tests/`.
6. **Permissão mínima.** Só os escopos `cobv.read`, `cobv.write` e `pix.read`. O `pix.read` é temporário (veja [contexto](docs/contexto.md)): o código precisa funcionar sem ele, e de `GET /pix` só se guardam os Pix sem `txid`. Nunca escopos de pagamento, devolução (`pix.write`) ou transferência.
7. **Nunca dados reais de pessoas.** Testes e exemplos usam dados inventados.

## Convenções

- **Português** nos nomes de tabelas, colunas, funções e variáveis.
- **Dinheiro em centavos inteiros** (`valor_centavos`, com o tipo `Centavos` sobre `i64`). Nunca ponto flutuante. Conversão para o formato do banco ("125.00") só no adaptador.
- **Datas:** data sem hora para vencimento e período (sempre o dia 1 do mês); data e hora com fuso para eventos.
- **Migrações** em SQL puro, criadas com `sqlx migrate add` em `migrations/`, só para frente. Não edite uma migração que já foi para a `main` (o `sqlx` recusa a checksum diferente). Depois de mudar migração ou consulta, rode `cargo sqlx prepare` e commite o `.sqlx/`.
- **Consultas** com as macros `sqlx::query!`/`query_as!`, conferidas na compilação. Nada de SQL montado com `format!`.
- **Situações da cobrança** como `enum`; nada de `_ =>` em `match` sobre elas, para o compilador apontar todo lugar quando surgir uma nova.
- **Erros** com tipos próprios por contexto; nada de `unwrap()` ou `expect()` fora de testes.
- **Adaptador falso** (`BancoFalso`) imita o comportamento do Inter, inclusive erros, 429 e quedas. Mudou o adaptador real, atualize o falso. O falso segue o comportamento confirmado do Inter (documentação ou resposta gravada do sandbox), não o que supomos.

## Testes

- Toda mudança em emissão ou baixa precisa de teste de idempotência: rodar duas vezes, falha no meio, mesmo pagamento visto duas vezes.
- O adaptador Inter é testado contra respostas gravadas do sandbox, sem rede.
- Testes de isolamento: a organização A não lê nem escreve dados da B.

## Forma de trabalhar

- Uma issue por pull request, pequena o bastante para ser lida de uma vez.
- Explique no PR o porquê das decisões; o dono do projeto revisa para aprender.
- Documentação junto com o código: se o PR muda uma decisão ou um processo, atualize no mesmo PR a tabela de decisões em `docs/contexto.md` (com a data) e o diagrama em `docs/processos.md`.
- Na dúvida entre a issue e a documentação do banco, pare e pergunte em vez de adivinhar.
