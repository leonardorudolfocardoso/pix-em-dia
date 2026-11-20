//! Experimento da iteração 1 (docs/iteracoes.md): cria cobranças com
//! vencimento (`cobv`) no sandbox do Inter e mostra cada resposta, para
//! responder as issues #1 (dados do devedor) e #2 (`PUT` repetido).
//!
//! Não é código de produção: é um roteiro de chamadas, de cima para baixo,
//! para ler junto com a saída. O adaptador de verdade vem depois, sobre os
//! fatos que este programa mostrar.
//!
//! Credenciais só por variáveis de ambiente, nunca no repositório:
//!
//! ```bash
//! export INTER_CLIENT_ID=...          # da integração no sandbox
//! export INTER_CLIENT_SECRET=...
//! export INTER_CERTIFICADO=~/inter-sandbox/certificado.crt   # fora do repositório
//! export INTER_CHAVE_PRIVADA=~/inter-sandbox/chave.key
//! export INTER_CHAVE_PIX=...          # chave Pix da conta do sandbox
//! export INTER_CONTA_CORRENTE=...     # opcional: só se o Inter pedir o cabeçalho
//! cargo run --example sandbox
//! ```
//!
//! O sandbox só funciona em dia útil, das 8h às 20h. Só dados inventados.

use std::env;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Days, Local};
use pix_em_dia::calcular_txid;
use reqwest::{Client, Identity, Method, RequestBuilder};
use serde_json::{Value, json};
use uuid::Uuid;

const SANDBOX: &str = "https://cdpj-sandbox.partners.uatinter.co";

type Resultado<T> = Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Resultado<()> {
    let inter = Inter::conectar().await?;

    // Cada execução usa txids novos: uma "organização" inventada a partir da
    // hora, passada pelo mesmo `calcular_txid` que a emissão vai usar.
    let execucao = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let organizacao = Uuid::from_u128(u128::from(execucao));
    let txid = |nome: &str| calcular_txid(organizacao, &format!("experimento:{nome}"));
    let vencimento = (Local::now().date_naive() + Days::new(10)).to_string();
    let cobranca = |valor: &str, devedor: Option<Value>| {
        let mut corpo = json!({
            "calendario": { "dataDeVencimento": vencimento, "validadeAposVencimento": 30 },
            "valor": { "original": valor },
            "chave": inter.chave_pix,
            "solicitacaoPagador": "Experimento Em Dia, dados inventados",
        });
        if let Some(devedor) = devedor {
            corpo["devedor"] = devedor;
        }
        corpo
    };

    titulo("Issue #1: dados do devedor");
    let sem_devedor = txid("sem-devedor");
    let so_nome = txid("so-nome");
    let nome_e_cpf = txid("nome-e-cpf");
    let criadas = [
        (
            &sem_devedor,
            inter
                .put_cobv("1a. sem devedor", &sem_devedor, cobranca("1.00", None))
                .await?,
        ),
        (
            &so_nome,
            inter
                .put_cobv(
                    "1b. devedor só com nome",
                    &so_nome,
                    cobranca("1.00", Some(json!({ "nome": "Fulano de Tal" }))),
                )
                .await?,
        ),
        (
            &nome_e_cpf,
            inter
                .put_cobv(
                    "1c. devedor com nome e CPF inventado",
                    &nome_e_cpf,
                    // 123.456.789-09: CPF de exemplo, com dígitos verificadores
                    // válidos e de ninguém.
                    cobranca(
                        "1.00",
                        Some(json!({ "nome": "Fulano de Tal", "cpf": "12345678909" })),
                    ),
                )
                .await?,
        ),
    ];

    // A #2 usa a primeira forma que o sandbox aceitou, com o menos de dado
    // pessoal possível.
    let Some((txid, devedor)) = criadas
        .iter()
        .zip([
            None,
            Some(json!({ "nome": "Fulano de Tal" })),
            Some(json!({ "nome": "Fulano de Tal", "cpf": "12345678909" })),
        ])
        .find_map(|((txid, aceita), devedor)| aceita.then_some(((*txid).clone(), devedor)))
    else {
        println!("\nO sandbox recusou as três formas; a #2 fica para depois.");
        return Ok(());
    };

    titulo("Issue #2: PUT repetido no mesmo txid");
    inter
        .get_cobv("2a. situação depois de criar", &txid)
        .await?;
    inter
        .put_cobv(
            "2b. o mesmo PUT de novo",
            &txid,
            cobranca("1.00", devedor.clone()),
        )
        .await?;
    inter
        .get_cobv("2c. situação depois do PUT repetido", &txid)
        .await?;
    inter
        .put_cobv(
            "2d. PUT com outro valor",
            &txid,
            cobranca("2.00", devedor.clone()),
        )
        .await?;
    inter.get_cobv("2e. o valor mudou?", &txid).await?;
    inter
        .chamar(
            "2f. remover (PATCH com status)",
            Method::PATCH,
            &format!("/pix/v2/cobv/{txid}"),
            Some(json!({ "status": "REMOVIDA_PELO_USUARIO_RECEBEDOR" })),
        )
        .await?;
    inter
        .put_cobv(
            "2g. PUT depois de removida",
            &txid,
            cobranca("1.00", devedor),
        )
        .await?;
    inter.get_cobv("2h. situação final", &txid).await?;

    println!("\nFalta: PUT numa cobrança paga, se o sandbox tiver como simular o pagamento.");
    Ok(())
}

struct Inter {
    http: Client,
    token: String,
    conta_corrente: Option<String>,
    chave_pix: String,
}

impl Inter {
    /// Monta o cliente com o certificado (mTLS) e pede o token OAuth.
    async fn conectar() -> Resultado<Self> {
        let mut pem = std::fs::read(variavel("INTER_CERTIFICADO")?)?;
        pem.extend(std::fs::read(variavel("INTER_CHAVE_PRIVADA")?)?);
        let http = Client::builder()
            .identity(Identity::from_pem(&pem)?)
            .build()?;

        titulo("Token");
        let resposta = http
            .post(format!("{SANDBOX}/oauth/v2/token"))
            .form(&[
                ("client_id", variavel("INTER_CLIENT_ID")?),
                ("client_secret", variavel("INTER_CLIENT_SECRET")?),
                ("grant_type", "client_credentials".to_string()),
                ("scope", "cobv.read cobv.write".to_string()),
            ])
            .send()
            .await?;
        let status = resposta.status();
        let corpo: Value = resposta.json().await?;
        // Mostra tudo menos o próprio token.
        let mut visivel = corpo.clone();
        if let Some(token) = visivel.get_mut("access_token") {
            *token = json!("(escondido)");
        }
        println!("POST /oauth/v2/token -> {status}\n{visivel:#}");
        let token = corpo["access_token"]
            .as_str()
            .ok_or("a resposta do token não trouxe access_token")?
            .to_string();

        Ok(Self {
            http,
            token,
            conta_corrente: env::var("INTER_CONTA_CORRENTE").ok(),
            chave_pix: variavel("INTER_CHAVE_PIX")?,
        })
    }

    /// `PUT /cobv/{txid}`. Devolve se o sandbox aceitou (2xx).
    async fn put_cobv(&self, passo: &str, txid: &str, corpo: Value) -> Resultado<bool> {
        let caminho = format!("/pix/v2/cobv/{txid}");
        self.chamar(passo, Method::PUT, &caminho, Some(corpo)).await
    }

    async fn get_cobv(&self, passo: &str, txid: &str) -> Resultado<bool> {
        let caminho = format!("/pix/v2/cobv/{txid}");
        self.chamar(passo, Method::GET, &caminho, None).await
    }

    /// Faz a chamada, mostra o status e a resposta e devolve se foi 2xx.
    async fn chamar(
        &self,
        passo: &str,
        metodo: Method,
        caminho: &str,
        corpo: Option<Value>,
    ) -> Resultado<bool> {
        let mut pedido: RequestBuilder = self
            .http
            .request(metodo.clone(), format!("{SANDBOX}{caminho}"))
            .bearer_auth(&self.token);
        if let Some(conta) = &self.conta_corrente {
            pedido = pedido.header("x-conta-corrente", conta);
        }
        if let Some(corpo) = &corpo {
            pedido = pedido.json(corpo);
        }
        let resposta = pedido.send().await?;
        let status = resposta.status();
        let texto = resposta.text().await?;
        let legivel = serde_json::from_str::<Value>(&texto)
            .map(|json| format!("{json:#}"))
            .unwrap_or(texto);
        println!("\n## {passo}\n{metodo} {caminho} -> {status}\n{legivel}");
        Ok(status.is_success())
    }
}

fn variavel(nome: &str) -> Resultado<String> {
    env::var(nome).map_err(|_| format!("falta a variável de ambiente {nome}").into())
}

fn titulo(texto: &str) {
    println!("\n# {texto}");
}
