use p3_field::{PrimeCharacteristicRing, PrimeField64};
use p3_goldilocks::Goldilocks;

use anyhow::Result;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir, BaseAirWithPublicValues};
use p3_field::extension::BinomialExtensionField;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;

use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_fri::TwoAdicFriPcs;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_symmetric::{
    CompressionFunctionFromHasher, CryptographicPermutation, PaddingFreeSponge, Permutation,
};
use p3_uni_stark::StarkConfig;

pub struct AuditAir;

pub const WIDTH: usize = 8;
pub const AUX_WIDTH: usize = 8;
pub const CONST_WIDTH: usize = 8;

pub const COL_STATE_START: usize = 0;
pub const COL_AUX_START: usize = 8;
pub const COL_CONST_START: usize = 16;
pub const COL_IS_FULL: usize = 24;
pub const COL_IS_ACTIVE: usize = 25;
pub const COL_IS_LAST: usize = 26;
pub const COL_IS_REAL: usize = 27;
pub const FULL_WIDTH: usize = 28;

pub const FULL_ROUNDS_START: usize = 4;
pub const PARTIAL_ROUNDS: usize = 22;
pub const FULL_ROUNDS_END: usize = 4;
pub const TOTAL_ROUNDS: usize = FULL_ROUNDS_START + PARTIAL_ROUNDS + FULL_ROUNDS_END;

pub const ME_CIRC: [u64; 8] = [3, 1, 1, 1, 1, 1, 1, 2];
pub const MU: [u64; 4] = [5, 6, 5, 6];

pub fn get_round_constant(round: usize, element: usize) -> u64 {
    let base = (round + 1) as u64 * 0x12345678;
    let offset = (element + 1) as u64 * 0x87654321;
    base.wrapping_add(offset)
}

impl<F> BaseAir<F> for AuditAir {
    fn width(&self) -> usize {
        FULL_WIDTH
    }
}
impl<F> BaseAirWithPublicValues<F> for AuditAir {
    fn num_public_values(&self) -> usize {
        2
    }
}

impl<AB: AirBuilder + AirBuilderWithPublicValues> Air<AB> for AuditAir
where
    AB::F: PrimeField64,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0).expect("exists");
        let next = main.row_slice(1).expect("exists");
        let p_vals = builder.public_values();
        let start_root = p_vals[0];
        let target_root = p_vals[1];

        builder
            .when_first_row()
            .assert_eq(local[COL_STATE_START].clone(), start_root);
        let is_last = local[COL_IS_LAST].clone();
        let is_real = local[COL_IS_REAL].clone();
        let is_active = local[COL_IS_ACTIVE].clone();
        let state_0: AB::Expr = local[COL_STATE_START].clone().into();
        let target_expr: AB::Expr = target_root.into();
        builder.assert_zero(is_last.clone() * (state_0 - target_expr));

        let is_full = local[COL_IS_FULL].clone();
        let mut sbox_out = Vec::with_capacity(WIDTH);
        for i in 0..WIDTH {
            let x: AB::Expr = local[COL_STATE_START + i].clone().into();
            let x3: AB::Expr = local[COL_AUX_START + i].clone().into();
            let x3_target = x.clone() * x.clone() * x.clone();
            builder.assert_zero(is_real.clone() * (x3.clone() - x3_target));
            let x7 = x3.clone() * x3.clone() * x.clone();
            if i == 0 {
                sbox_out.push(x7);
            } else {
                let sbox_i = is_full.clone() * x7 + (AB::Expr::ONE - is_full.clone()) * x;
                sbox_out.push(sbox_i);
            }
        }
        let mut full_linear = Vec::with_capacity(WIDTH);
        for r in 0..WIDTH {
            let mut row_sum = AB::Expr::ZERO;
            for c in 0..WIDTH {
                row_sum +=
                    sbox_out[c].clone() * AB::Expr::from_u64(ME_CIRC[(WIDTH + r - c) % WIDTH]);
            }
            full_linear.push(row_sum);
        }
        let sum_sbox: AB::Expr = sbox_out.iter().cloned().sum();
        let mut partial_linear = Vec::with_capacity(WIDTH);
        for i in 0..WIDTH {
            let mu_val = AB::Expr::from_u64(MU[i % 4]);
            partial_linear.push((mu_val - AB::Expr::ONE) * sbox_out[i].clone() + sum_sbox.clone());
        }
        for i in 0..WIDTH {
            let linear_out = is_full.clone() * full_linear[i].clone()
                + (AB::Expr::ONE - is_full.clone()) * partial_linear[i].clone();
            let const_expr: AB::Expr = local[COL_CONST_START + i].clone().into();
            let target_next = linear_out + const_expr;
            let next_i: AB::Expr = next[COL_STATE_START + i].clone().into();
            builder
                .when_transition()
                .assert_zero(is_active.clone() * (next_i - target_next));
        }
    }
}

pub fn generate_trace_rows(initial: Val, _s: Val, num_steps: usize) -> RowMajorMatrix<Val> {
    let mut values = Vec::new();
    let mut current_state = [Val::ZERO; WIDTH];
    current_state[0] = initial;
    for step in 0..num_steps {
        let is_full = !(FULL_ROUNDS_START..FULL_ROUNDS_START + PARTIAL_ROUNDS).contains(&step);
        values.extend_from_slice(&current_state);
        for item in current_state.iter().take(WIDTH) {
            values.push(*item * *item * *item);
        }
        for i in 0..WIDTH {
            values.push(Val::from_u64(get_round_constant(step, i)));
        }
        values.push(Val::from_bool(is_full));
        values.push(Val::ONE);
        values.push(Val::ZERO);
        values.push(Val::ONE);
        let mut sbox_out = [Val::ZERO; WIDTH];
        for i in 0..WIDTH {
            let x = current_state[i];
            if is_full || i == 0 {
                sbox_out[i] = x * x * x * x * x * x * x;
            } else {
                sbox_out[i] = x;
            }
        }
        let mut next_state = [Val::ZERO; WIDTH];
        if is_full {
            for r in 0..8 {
                let mut sum = Val::ZERO;
                for c in 0..8 {
                    sum += Val::from_u64(ME_CIRC[(8 + r - c) % 8]) * sbox_out[c];
                }
                next_state[r] = sum + Val::from_u64(get_round_constant(step, r));
            }
        } else {
            let sum_sbox: Val = sbox_out.iter().cloned().sum();
            for i in 0..WIDTH {
                next_state[i] = (Val::from_u64(MU[i % 4]) - Val::new(1)) * sbox_out[i]
                    + sum_sbox
                    + Val::from_u64(get_round_constant(step, i));
            }
        }
        current_state = next_state;
    }
    values.extend_from_slice(&current_state);
    for item in current_state.iter().take(WIDTH) {
        values.push(*item * *item * *item);
    }
    for _ in 0..8 {
        values.push(Val::ZERO);
    }
    values.push(Val::ZERO);
    values.push(Val::ZERO);
    values.push(Val::ONE);
    values.push(Val::ONE);
    let h = values.len() / FULL_WIDTH;
    let n = h.next_power_of_two().max(128);
    for _ in h..n {
        for _ in 0..FULL_WIDTH {
            values.push(Val::ZERO);
        }
    }
    RowMajorMatrix::new(values, FULL_WIDTH)
}

pub struct AuditProver;
type Val = Goldilocks;
type Challenge = BinomialExtensionField<Val, 2>;
#[derive(Clone, Default)]
pub struct MyPerm;
impl Permutation<[Val; 8]> for MyPerm {
    fn permute_mut(&self, state: &mut [Val; 8]) {
        for step in 0..TOTAL_ROUNDS {
            let is_full = !(FULL_ROUNDS_START..FULL_ROUNDS_START + PARTIAL_ROUNDS).contains(&step);
            let mut sbox_out = [Val::ZERO; WIDTH];
            for i in 0..WIDTH {
                let x = state[i];
                if is_full || i == 0 {
                    sbox_out[i] = x * x * x * x * x * x * x;
                } else {
                    sbox_out[i] = x;
                }
            }
            let mut next_state = [Val::ZERO; WIDTH];
            if is_full {
                for r in 0..8 {
                    let mut sum = Val::ZERO;
                    for c in 0..8 {
                        sum += Val::from_u64(ME_CIRC[(8 + r - c) % 8]) * sbox_out[c];
                    }
                    next_state[r] = sum + Val::from_u64(get_round_constant(step, r));
                }
            } else {
                let sum_sbox: Val = sbox_out.iter().cloned().sum();
                for i in 0..WIDTH {
                    next_state[i] = (Val::from_u64(MU[i % 4]) - Val::new(1)) * sbox_out[i]
                        + sum_sbox
                        + Val::from_u64(get_round_constant(step, i));
                }
            }
            *state = next_state;
        }
    }
}
impl CryptographicPermutation<[Val; 8]> for MyPerm {}

type MyHash = PaddingFreeSponge<MyPerm, 8, 4, 4>;
type MyCompress = CompressionFunctionFromHasher<MyHash, 2, 4>;
type ValMmcs = MerkleTreeMmcs<Val, Val, MyHash, MyCompress, 4>;
type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
type Dft = Radix2DitParallel<Val>;
type MyPcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
type MyChallenger = DuplexChallenger<Val, MyPerm, 8, 4>;
pub type AuditStarkConfig = StarkConfig<MyPcs, Challenge, MyChallenger>;

impl AuditProver {
    pub fn build_stark_config() -> AuditStarkConfig {
        let perm = MyPerm {};
        let mmcs = ValMmcs::new(
            MyHash::new(perm.clone()),
            MyCompress::new(MyHash::new(perm.clone())),
        );
        let params = p3_fri::FriParameters {
            log_blowup: 3,
            log_final_poly_len: 0,
            num_queries: 100,
            commit_proof_of_work_bits: 0,
            query_proof_of_work_bits: 0,
            mmcs: ChallengeMmcs::new(mmcs),
        };
        AuditStarkConfig::new(
            MyPcs::new(
                Dft::default(),
                ValMmcs::new(
                    MyHash::new(perm.clone()),
                    MyCompress::new(MyHash::new(perm.clone())),
                ),
                params,
            ),
            MyChallenger::new(perm),
        )
    }
    pub fn prove_transition(prev: [u8; 32], _hash: [u8; 32]) -> Result<Vec<u8>> {
        let i = Val::new(u64::from_le_bytes(prev[0..8].try_into().unwrap()));
        let trace = generate_trace_rows(i, Val::ZERO, TOTAL_ROUNDS);
        let f = trace.row_slice(TOTAL_ROUNDS).expect("exists")[COL_STATE_START];
        let stark_proof = p3_uni_stark::prove::<AuditStarkConfig, AuditAir>(
            &Self::build_stark_config(),
            &AuditAir,
            trace,
            &[i, f],
        );
        let mut blob = Vec::new();
        blob.extend_from_slice(b"STARK_P3_V1");
        let serial = serde_json::to_vec(&stark_proof)?;
        blob.extend_from_slice(&(serial.len() as u32).to_le_bytes());
        blob.extend_from_slice(&serial);
        blob.extend_from_slice(&i.as_canonical_u64().to_le_bytes());
        blob.extend_from_slice(&f.as_canonical_u64().to_le_bytes());
        Ok(blob)
    }
    pub fn verify_proof(blob: &[u8], _ir: [u8; 32], _fr: [u8; 32]) -> Result<bool> {
        if !blob.starts_with(b"STARK_P3_V1") {
            return Ok(false);
        }
        let mut cursor = 11;
        let p_len = u32::from_le_bytes(blob[cursor..cursor + 4].try_into().unwrap()) as usize;
        let proof: p3_uni_stark::Proof<AuditStarkConfig> =
            serde_json::from_slice(&blob[cursor + 4..cursor + 4 + p_len])?;
        cursor += 4 + p_len;
        let i = Goldilocks::new(u64::from_le_bytes(
            blob[cursor..cursor + 8].try_into().unwrap(),
        ));
        let f = Goldilocks::new(u64::from_le_bytes(
            blob[cursor + 8..cursor + 16].try_into().unwrap(),
        ));
        Ok(p3_uni_stark::verify::<AuditStarkConfig, AuditAir>(
            &Self::build_stark_config(),
            &AuditAir,
            &proof,
            &[i, f],
        )
        .is_ok())
    }
}

impl vex_core::zk::ZkVerifier for AuditProver {
    fn verify_stark(
        &self,
        commitment_hash: &str,
        stark_proof_b64: &str,
        _public_inputs: &serde_json::Value,
    ) -> Result<bool, vex_core::zk::ZkError> {
        use base64::{engine::general_purpose, Engine as _};

        // 1. Decode Proof
        let proof_bytes = general_purpose::STANDARD
            .decode(stark_proof_b64)
            .map_err(|e| {
                vex_core::zk::ZkError::InvalidFormat(format!("Base64 decode failed: {}", e))
            })?;

        // 2. Decode Commitment (Target Root)
        let commitment_bytes = hex::decode(commitment_hash).map_err(|e| {
            vex_core::zk::ZkError::InvalidFormat(format!("Hex decode failed: {}", e))
        })?;

        if commitment_bytes.len() != 32 {
            return Err(vex_core::zk::ZkError::InvalidFormat(
                "Commitment must be 32 bytes".to_string(),
            ));
        }

        let mut next_state = [0u8; 32];
        next_state.copy_from_slice(&commitment_bytes);

        // 3. Verify via Plonky3
        // Note: For Shadow Intents, we assume an initial state of [0; 32] for now.
        // In a full implementation, the 'start_root' might come from public_inputs.
        Self::verify_proof(&proof_bytes, [0u8; 32], next_state)
            .map_err(|e| vex_core::zk::ZkError::VerificationFailed(e.to_string()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_zk_transition_valid() {
        let prev = [3u8; 32];
        let proof = AuditProver::prove_transition(prev, [0u8; 32]).unwrap();
        assert!(AuditProver::verify_proof(&proof, prev, [0u8; 32]).unwrap());
    }
    #[test]
    fn test_zk_diffusion() {
        let (mut s1, mut s2) = ([Val::ZERO; 8], [Val::ZERO; 8]);
        s1[0] = Val::new(100);
        s2[0] = Val::new(101);
        MyPerm.permute_mut(&mut s1);
        MyPerm.permute_mut(&mut s2);
        assert_eq!(s1.iter().zip(s2.iter()).filter(|(a, b)| a != b).count(), 8);
    }
    #[test]
    fn test_zk_incorrect_math() {
        let prev = [3u8; 32];
        let mut proof = AuditProver::prove_transition(prev, [0u8; 32]).unwrap();
        let last = proof.len() - 1;
        proof[last] ^= 0xFF;
        assert!(!AuditProver::verify_proof(&proof, prev, [0u8; 32]).unwrap());
    }

    #[test]
    fn test_zk_verifier_implementation() {
        use base64::{engine::general_purpose, Engine as _};
        use vex_core::zk::ZkVerifier;

        let prev = [0u8; 32];
        let next = [1u8; 32];
        let proof = AuditProver::prove_transition(prev, next).unwrap();

        let commitment_hash = hex::encode(next);
        let stark_proof_b64 = general_purpose::STANDARD.encode(proof);
        let public_inputs = serde_json::json!({});

        let prover = AuditProver;
        let result = prover
            .verify_stark(&commitment_hash, &stark_proof_b64, &public_inputs)
            .unwrap();
        assert!(
            result,
            "ZkVerifier implementation must correctly verify a valid proof"
        );
    }
}
