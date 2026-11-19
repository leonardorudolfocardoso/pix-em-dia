//! `txid` determinístico da cobrança Pix (roteiro E1.5).
//!
//! O `txid` sai da organização e da chave de idempotência, nunca de um
//! sorteio: pedir a mesma cobrança de novo dá o mesmo `txid`, e o
//! `PUT /cobv/{txid}` repetido cai na mesma cobrança no banco.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Quantos bytes do SHA-256 viram `txid`.
///
/// - **Formato:** cada byte vira 2 caracteres hexadecimais, então 16 bytes
///   dão 32 caracteres, dentro dos 26 a 35 que o Banco Central aceita. O
///   `assert!` abaixo confere isso na compilação.
/// - **Colisão:** 16 bytes são 128 bits. Pelo paradoxo do aniversário, a
///   chance de dois de `n` txids coincidirem é no máximo `n² / 2¹²⁹`. Com
///   um milhão de cobranças numa organização (uma escola de 500 alunos leva
///   mais de 150 anos para chegar lá), dá `10¹² / 6,8 × 10³⁸ ≈ 1,5 × 10⁻²⁷`.
///   E o `UNIQUE (organizacao_id, txid)` do banco recusaria a colisão em vez
///   de misturar cobranças.
const BYTES_DO_TXID: usize = 16;

const _: () = assert!(2 * BYTES_DO_TXID >= 26 && 2 * BYTES_DO_TXID <= 35);

/// Calcula o `txid` de uma cobrança: os 32 primeiros caracteres, em
/// hexadecimal minúsculo, do SHA-256 do id da organização (16 bytes) seguido
/// da chave de idempotência.
///
/// O id da organização tem tamanho fixo, então nenhuma combinação de
/// organização e chave se confunde com outra. O hash não revela a chave nem
/// dado pessoal, e só usa `0-9a-f`, sem depender de maiúscula ou minúscula.
pub fn calcular_txid(organizacao: Uuid, chave_idempotencia: &str) -> String {
    let hash = Sha256::new()
        .chain_update(organizacao.as_bytes())
        .chain_update(chave_idempotencia.as_bytes())
        .finalize();
    // `take` em vez de `hash[..BYTES_DO_TXID]`: nunca entra em pânico. O
    // SHA-256 sempre tem 32 bytes, então os 16 pedidos sempre existem.
    hash.iter()
        .take(BYTES_DO_TXID)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORG_A: Uuid = Uuid::from_u128(0x0193_6a1e_7c2b_7d4f_9e8a_1b2c_3d4e_5f60);
    const ORG_B: Uuid = Uuid::from_u128(0x0193_6a1e_7c2b_7d4f_9e8a_1b2c_3d4e_5f61);

    /// Confere a regra do Banco Central para o `txid` da `cobv`:
    /// `^[a-zA-Z0-9]{26,35}$`.
    fn eh_txid_valido(txid: &str) -> bool {
        (26..=35).contains(&txid.len()) && txid.bytes().all(|c| c.is_ascii_alphanumeric())
    }

    #[test]
    fn mesma_entrada_da_mesmo_txid() {
        let chave = "mensal:assinatura-1:2026-12";
        assert_eq!(calcular_txid(ORG_A, chave), calcular_txid(ORG_A, chave));
    }

    #[test]
    fn txid_nao_muda_entre_versoes() {
        // Cobranças já emitidas guardam este txid; se o cálculo mudar, a
        // próxima rotina emitiria uma cobrança nova em vez de reencontrar a
        // antiga. Este valor só pode mudar com uma migração dos txids.
        assert_eq!(
            calcular_txid(ORG_A, "mensal:assinatura-1:2026-12"),
            "d13af7e80fa2c7351ea3db82d172cbb1"
        );
    }

    #[test]
    fn chaves_diferentes_dao_txids_diferentes() {
        let chaves = [
            "mensal:assinatura-1:2026-12",
            "mensal:assinatura-1:2027-01",
            "mensal:assinatura-2:2026-12",
            "evento-2026:aluno-7",
            "planilha:42:assinatura-1",
            "",
        ];
        let mut txids: Vec<_> = chaves.iter().map(|c| calcular_txid(ORG_A, c)).collect();
        txids.sort();
        txids.dedup();
        assert_eq!(txids.len(), chaves.len());
    }

    #[test]
    fn organizacoes_diferentes_dao_txids_diferentes() {
        let chave = "mensal:assinatura-1:2026-12";
        assert_ne!(calcular_txid(ORG_A, chave), calcular_txid(ORG_B, chave));
    }

    #[test]
    fn formato_sempre_valido() {
        let chaves = [
            "",
            "mensal:assinatura-1:2026-12",
            "acentuação e espaços: São João",
            &"x".repeat(10_000),
        ];
        for chave in chaves {
            for org in [ORG_A, ORG_B, Uuid::nil(), Uuid::max()] {
                let txid = calcular_txid(org, chave);
                assert!(eh_txid_valido(&txid), "txid inválido: {txid}");
            }
        }
    }
}
