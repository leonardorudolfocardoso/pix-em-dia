# Iterações

O Em Dia cresce em iterações curtas. Cada uma termina com algo que roda de ponta a ponta e responde perguntas com fatos (respostas reais do banco), não com suposições. Depois de cada iteração, o [roteiro](roteiro.md), que é um backlog, é revisado com o que se aprendeu: itens mudam, somem ou trocam de ordem.

As issues abertas no GitHub são as perguntas de uma iteração. Uma iteração só fecha quando as perguntas dela estão respondidas, com a fonte, em [contexto.md](contexto.md).

## Iteração 1: uma cobrança no sandbox do Inter

**Objetivo:** criar uma cobrança Pix com vencimento (`cobv`) no sandbox do Inter e ler de volta a situação dela, com o mínimo de código. É o risco maior do projeto, e o roteiro só chegava a ele na etapa 2.

**Perguntas que responde:**

- [#1](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/1) Dados do devedor: a `cobv` aceita cobrança sem devedor? Só com nome? Exige CPF?
- [#2](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/2) Idempotência: o que um segundo `PUT` no mesmo `txid` faz numa cobrança ativa, numa paga e numa removida?
- Parte da [#3](https://github.com/leonardorudolfocardoso/pix-em-dia/issues/3): endereço do sandbox, endpoint e validade do token, nomes dos escopos, se a resposta traz `pixCopiaECola`, e se dá para simular um pagamento de `cobv`.

**Inclui:**

1. **Tarefa manual (dono do projeto):** conta na comunidade de desenvolvedores do Inter, integração no sandbox com os escopos `cobv.read` e `cobv.write`, certificado e chave baixados para uma pasta fora do repositório, e a chave Pix do sandbox anotada.
2. **Programa de experimento** (`cargo run --example sandbox`): pede o token com o certificado (mTLS) e roda os experimentos abaixo, mostrando o status HTTP e a resposta de cada chamada. Lê as credenciais de variáveis de ambiente; nada de segredo no código. Roda no computador do dono do projeto, onde estão o certificado e a chave, em dia útil das 8h às 20h (horário do sandbox).
   - **#1:** `PUT /cobv/{txid}` sem `devedor`; com `devedor` só com nome; com nome e um CPF inventado válido.
   - **#2:** o mesmo `PUT` duas vezes; o segundo com outro valor; `PUT` depois de remover a cobrança (`PATCH` com `status` `REMOVIDA_PELO_USUARIO_RECEBEDOR`); `PUT` depois de paga, se o sandbox simular pagamento.
   - **Leitura:** `GET /cobv/{txid}` depois de cada passo.
3. **Registro:** as respostas, sem dados de pessoas (só dados inventados) e sem credenciais, viram fatos em [contexto.md](contexto.md) com a data, e as decisões das issues #1 e #2 entram na tabela de decisões.

**Fora:** banco de dados, `AdaptadorBanco` e `BancoFalso` (E1.6). O contrato da interface ("emitir de novo o mesmo `txid` devolve a cobrança existente") depende da resposta da #2, então ela vem depois, já sobre fatos. Um rascunho existe e é retomado na iteração seguinte.

**Pronta quando:** uma cobrança criada pelo programa aparece no sandbox, as issues #1 e #2 estão respondidas e fechadas, e o roteiro foi revisado com o que se aprendeu.
