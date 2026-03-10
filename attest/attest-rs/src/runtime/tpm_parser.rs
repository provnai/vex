use anyhow::{anyhow, Result};
use std::convert::TryInto;

/// TPM 2.0 Magic Value (TPM_GENERATED)
pub const TPM_GENERATED_VALUE: u32 = 0xFF544347;

/// TPM 2.0 Quote Type (TPM_ST_ATTEST_QUOTE)
pub const TPM_ST_ATTEST_QUOTE: u16 = 0x8018;

/// Partial TPMS_ATTEST structure
#[derive(Debug, Clone)]
pub struct TpmsAttest {
    pub magic: u32,
    pub type_: u16,
    pub extra_data: Vec<u8>,
    pub pcr_select: Vec<u8>,
    pub pcr_digest: Vec<u8>,
    /// The raw bytes of the TPMS_ATTEST structure that the TPM signs.
    /// Signature verification is performed over this buffer.
    pub attested_bytes: Vec<u8>,
}

impl TpmsAttest {
    /// Parse a TPMS_ATTEST from a marshalling buffer.
    /// This is a simplified parser for specific fields we need.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut offset = 0;

        // 1. Magic
        let magic = u32::from_be_bytes(data[offset..offset + 4].try_into()?);
        offset += 4;
        if magic != TPM_GENERATED_VALUE {
            return Err(anyhow!("Invalid TPM magic: 0x{:08X}", magic));
        }

        // 2. Type
        let type_ = u16::from_be_bytes(data[offset..offset + 2].try_into()?);
        offset += 2;
        if type_ != TPM_ST_ATTEST_QUOTE {
            return Err(anyhow!("Invalid TPM attest type: 0x{:04X}", type_));
        }

        // 3. qualifiedSigner (TPM2B_NAME)
        let name_size = u16::from_be_bytes(data[offset..offset + 2].try_into()?) as usize;
        offset += 2 + name_size;

        // 4. extraData (TPM2B_DATA)
        let extra_data_size = u16::from_be_bytes(data[offset..offset + 2].try_into()?) as usize;
        offset += 2;
        let extra_data = data[offset..offset + extra_data_size].to_vec();
        offset += extra_data_size;

        // 5. clockInfo (TPMS_CLOCK_INFO)
        // Clock (8), ResetCount (4), RestartCount (4), Safe (1)
        offset += 8 + 4 + 4 + 1;

        // 6. firmwareVersion (u64)
        offset += 8;

        // 7. attested (TPMU_ATTEST)
        // For Quote: pcrSelect (TPML_PCR_SELECTION) + pcrDigest (TPM2B_DIGEST)

        // pcrSelect Count (u32)
        let selection_count = u32::from_be_bytes(data[offset..offset + 4].try_into()?) as usize;
        offset += 4;

        let mut pcr_select = Vec::new();
        for _ in 0..selection_count {
            // Hash (u16), pcrSize (u8), pcrSelect (pcrSize bytes)
            let _hash_alg = u16::from_be_bytes(data[offset..offset + 2].try_into()?);
            let pcr_size = data[offset + 2] as usize;
            pcr_select.extend_from_slice(&data[offset..offset + 3 + pcr_size]);
            offset += 3 + pcr_size;
        }

        // pcrDigest (TPM2B_DIGEST)
        let digest_size = u16::from_be_bytes(data[offset..offset + 2].try_into()?) as usize;
        offset += 2;
        let pcr_digest = data[offset..offset + digest_size].to_vec();

        Ok(Self {
            magic,
            type_,
            extra_data,
            pcr_select,
            pcr_digest,
            attested_bytes: data.to_vec(), // full TPMS_ATTEST blob — TPM signs this
        })
    }
}

/// Windows PCP Platform Attestation Blob Parser
pub struct PcpAttestationBlob {
    pub magic: [u8; 4], // 'PLAT'
    pub attest: TpmsAttest,
    pub signature: Vec<u8>,
}

impl PcpAttestationBlob {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(anyhow!("PCP blob too short"));
        }

        let mut offset = 0;
        let magic: [u8; 4] = data[0..4].try_into()?;
        if &magic != b"PLAT" {
            return Err(anyhow!("Invalid PCP magic: {:?}", magic));
        }
        offset += 4;

        // Skip Size (4), Version (4), Instance (4)
        offset += 12;

        let attest_size = u32::from_le_bytes(data[offset..offset + 4].try_into()?) as usize;
        offset += 4;
        let attest_data = &data[offset..offset + attest_size];
        let attest = TpmsAttest::parse(attest_data)?;
        offset += attest_size;

        let signature_size = u32::from_le_bytes(data[offset..offset + 4].try_into()?) as usize;
        offset += 4;
        let signature = data[offset..offset + signature_size].to_vec();

        Ok(Self {
            magic,
            attest,
            signature,
        })
    }
}
