/// Transaction replay protection
use super::{ChainId, SigV, SigVLegacy};

/// Merge chain_id and signature V
pub fn encode(v: SigV, chain_id: Option<ChainId>) -> SigVLegacy {
  let replay: u64 = if let Some(n) = chain_id {
    35 + n * 2
  } else {
    27
  };
  v as u64 + replay
}

/// Returns standard v from replay protected legacy V
pub fn decode_v(v: SigVLegacy) -> SigV {
  if v == 27 {
    0
  } else if v == 28 {
    1
  } else if v > 35 {
    ((v - 1) % 2) as u8
  } else {
    4 //invalid value
  }
}

// return chain id from replay protected legacy V
pub fn decode_chain_id(v: SigVLegacy) -> Option<ChainId> {
  if v >= 35 {
    Some((v - 35) / 2 as u64)
  } else {
    None
  }
}
