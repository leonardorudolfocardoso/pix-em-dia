# Em Dia

Cobrança por Pix para pequenos negócios: mensalidades e cobranças avulsas, com baixa automática e lista diária de lembretes.

> Em desenvolvimento. Primeira versão com Banco Inter, a ser testada no sandbox.

## O que faz

- **Emite cobranças Pix com vencimento**, recorrentes (uma por assinatura por mês) ou avulsas (uma cobrança única, para uma pessoa ou um grupo).
- **Dá baixa sozinho:** consulta o banco a cada 15 minutos e marca o que foi pago.
- **Monta a lista do dia** com quem lembrar e a mensagem pronta, com o Pix copia e cola. Uma pessoa envia; o Em Dia não manda mensagens.

## Princípios

- **Nunca duplica.** Rodar qualquer rotina de novo não cria uma segunda cobrança.
- **Sem IA no caminho do dinheiro.** Emissão e baixa são código determinístico.
- **Cada organização só vê os próprios dados**, garantido pelo banco de dados (Row Level Security).
- **Credenciais nunca no repositório.** Certificados, chaves e segredos ficam num cofre.
- **Permissão mínima no banco:** só criar e consultar cobranças. Nenhuma permissão para pagar ou transferir.

## Como funciona

```
organização ── assinaturas ── cobranças ── pagamentos
                                  │
                         adaptador do banco (Inter)
```

O código fala com o banco por um adaptador com quatro operações: emitir, consultar, cancelar e listar pagamentos. Trocar ou adicionar um banco significa escrever um novo adaptador.

## Desenvolvimento

Ainda não há código. Regras e convenções para quem for escrever estão em [AGENTS.md](AGENTS.md).

## Documentação

- [Contexto e decisões](docs/contexto.md): o problema, as decisões tomadas e os fatos da API do banco.
- [Processos](docs/processos.md): mapa de contextos e diagramas de emissão, baixa, lembretes e ciclo da cobrança.
- [Roteiro](docs/roteiro.md): as etapas e issues de desenvolvimento.
