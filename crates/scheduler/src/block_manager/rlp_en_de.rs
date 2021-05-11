// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{BlockHeaderAndHash, GetBlockHeaders, NewBlock, NewBlockHash};
use core::{BlockBody, BlockHeader, BlockId, BlockNumber, Transaction, H160, H256, U256};

use keccak_hash::keccak;
use rlp::{DecoderError, Rlp, RlpStream};

pub fn encode_new_block_hashes(request: &[NewBlockHash]) -> Vec<u8> {
    let mut stream = RlpStream::new_list(request.len());

    for block in request {
        stream
            .begin_list(2)
            .append(&block.hash)
            .append(&block.number);
    }

    stream.out().to_vec()
}

pub fn decode_new_block_hashes(data: &[u8]) -> Result<Vec<NewBlockHash>, DecoderError> {
    let encoded_hashes = Rlp::new(data);
    let mut decoded_hashes = vec![];

    for ref encoded_hash in encoded_hashes.iter() {
        let hash_data = encoded_hash.at(0)?.data()?;
        if hash_data.len() != 32 {
            panic!("ENCODED_HASH {:?}", encoded_hash);
        }
        decoded_hashes.push(NewBlockHash {
            hash: H256::from_slice(encoded_hash.at(0)?.data()?),
            number: encoded_hash.val_at(1)?,
        })
    }

    Ok(decoded_hashes)
}

pub fn encode_get_block_headers(request: &GetBlockHeaders) -> Vec<u8> {
    let mut stream = RlpStream::new_list(4);

    match request.block_id {
        BlockId::Number(number) => stream.append(&number),
        BlockId::Hash(hash) => stream.append(&hash),
        BlockId::Latest => panic!("Please use blocks number or hash"),
    };

    stream.append(&request.max_headers).append(&request.skip);

    if request.reverse {
        stream.append(&1u8);
    } else {
        stream.append_empty_data();
    }

    stream.out().to_vec()
}

pub fn decode_get_block_headers(data: &[u8]) -> Result<GetBlockHeaders, DecoderError> {
    let rlp = Rlp::new(data);

    let block_id_rlp = rlp.at(0)?;
    let block_id = match block_id_rlp.size() {
        32 => BlockId::Hash(H256::from_slice(block_id_rlp.data()?)),
        _ => BlockId::Number(block_id_rlp.as_val::<BlockNumber>()?),
    };

    let max_headers = rlp.at(1)?.as_val::<u64>()?;
    let skip = rlp.at(2)?.as_val::<u64>()?;
    let reverse = rlp.at(3)?.as_val::<bool>()?;

    Ok(GetBlockHeaders::new(block_id, max_headers, skip, reverse))
}

fn encode_block_header(stream: &mut RlpStream, header: &BlockHeader) {
    stream
        .begin_list(15)
        .append(&header.parent_hash)
        .append(&header.ommers_hash)
        .append(&header.beneficiary_address)
        .append(&header.state_root)
        .append(&header.transactions_root)
        .append(&header.receipts_root)
        .append(&header.logs_bloom)
        .append(&header.difficulty)
        .append(&header.number)
        .append(&header.gas_limit)
        .append(&header.gas_used)
        .append(&header.timestamp)
        .append(&header.extra_data)
        .append(&header.mix_hash)
        .append(&header.nonce);
}

pub fn encode_block_headers(headers: &[BlockHeader]) -> Vec<u8> {
    let mut stream = RlpStream::new_list(headers.len());
    for header in headers {
        encode_block_header(&mut stream, &header);
    }
    stream.out().to_vec()
}

fn decode_block_header(header: &Rlp) -> Result<BlockHeader, DecoderError> {
    Ok(BlockHeader {
        parent_hash: H256::from_slice(header.at(0)?.data()?),
        ommers_hash: H256::from_slice(header.at(1)?.data()?),
        beneficiary_address: H160::from_slice(header.at(2)?.data()?),
        state_root: H256::from_slice(header.at(3)?.data()?),
        transactions_root: H256::from_slice(header.at(4)?.data()?),
        receipts_root: H256::from_slice(header.at(5)?.data()?),
        logs_bloom: header.val_at(6)?,
        difficulty: header.val_at(7)?,
        number: header.val_at(8)?,
        gas_limit: header.val_at(9)?,
        gas_used: header.val_at(10)?,
        timestamp: header.val_at(11)?,
        extra_data: header.val_at(12)?,
        mix_hash: H256::from_slice(header.at(13)?.data()?),
        nonce: header.val_at(14)?,
    })
}

pub fn decode_block_headers(data: &[u8]) -> Result<Vec<BlockHeader>, DecoderError> {
    let encoded_headers = Rlp::new(data);
    let mut decoded_headers = vec![];
    for header in encoded_headers.iter() {
        decoded_headers.push(decode_block_header(&header)?);
    }
    Ok(decoded_headers)
}

pub fn decode_block_headers_with_hash(
    data: &[u8],
) -> Result<Vec<BlockHeaderAndHash>, DecoderError> {
    let encoded_headers = Rlp::new(data);
    let mut decoded_headers = vec![];
    for item in encoded_headers.iter() {
        let keccak_hash = keccak(item.as_raw());
        // FIXME why are there two different H256 types? (keccak/primitive_types version conflict?)
        let hash = H256::from_slice(keccak_hash.as_bytes());
        let header = decode_block_header(&item)?;
        decoded_headers.push(BlockHeaderAndHash { header, hash });
    }
    Ok(decoded_headers)
}

pub fn encode_get_block_bodies(hashes: &[H256]) -> Vec<u8> {
    let mut stream = RlpStream::new_list(hashes.len());
    for hash in hashes {
        stream.append(hash);
    }
    stream.out().to_vec()
}

pub fn decode_get_block_bodies(data: &[u8]) -> Result<Vec<H256>, DecoderError> {
    let rlp = Rlp::new(data);
    let mut hashes = vec![];
    for item in rlp.iter() {
        hashes.push(H256::from_slice(item.data()?));
    }
    Ok(hashes)
}

fn encode_block_body(stream: &mut RlpStream, block_body: &BlockBody) {
    let block_stream = stream.begin_list(2);
    Transaction::rlp_append_list(block_stream, &block_body.transactions);
    let mut ommers_stream = block_stream.begin_list(block_body.ommers.len());
    for ref ommer in &block_body.ommers {
        encode_block_header(&mut ommers_stream, ommer);
    }
}

pub fn encode_block_bodies(block_bodies: &[BlockBody]) -> Vec<u8> {
    let mut stream = RlpStream::new_list(block_bodies.len());
    for block_body in block_bodies {
        encode_block_body(&mut stream, &block_body);
    }
    stream.out().to_vec()
}

fn decode_block_body(body: &Rlp) -> Result<BlockBody, DecoderError> {
    let transactions = Transaction::rlp_decode_list(&body.at(0)?)?;
    let mut ommers = vec![];
    for ref ommer in body.at(1)?.iter() {
        ommers.push(decode_block_header(ommer)?);
    }
    Ok(BlockBody {
        transactions,
        ommers,
    })
}

pub fn decode_block_bodies(data: &[u8]) -> Result<Vec<BlockBody>, DecoderError> {
    let encoded_bodies = Rlp::new(data);
    let mut decoded_bodies = vec![];
    for ref body in encoded_bodies.iter() {
        decoded_bodies.push(decode_block_body(body)?);
    }
    Ok(decoded_bodies)
}

pub fn encode_new_block(new_block: &NewBlock) -> Vec<u8> {
    let mut stream = RlpStream::new_list(2);
    let mut first_part = stream.begin_list(3);

    encode_block_header(&mut first_part, &new_block.header);
    Transaction::rlp_append_list(first_part, &new_block.transactions);

    let mut ommer_stream = first_part.begin_list(new_block.ommers.len());
    for ref ommer in &new_block.ommers {
        encode_block_header(&mut ommer_stream, ommer);
    }

    stream.append(&new_block.score);

    stream.out().to_vec()
}

pub fn decode_new_block(data: &[u8]) -> Result<NewBlock, DecoderError> {
    let encoded = Rlp::new(data);

    let header = decode_block_header(&encoded.at(0)?.at(0)?)?;

    let transactions = Transaction::rlp_decode_list(&encoded)?;

    let mut ommers = vec![];
    for ref ommer in encoded.at(0)?.at(2)?.iter() {
        ommers.push(decode_block_header(ommer)?);
    }

    let score = U256::from_big_endian(&encoded.at(1)?.data()?);

    Ok(NewBlock {
        header,
        transactions,
        ommers,
        score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_block_hashes_roundtrip() {
        let request = vec![
            NewBlockHash::new(H256::repeat_byte(0x10), 42),
            NewBlockHash::new(H256::repeat_byte(0x22), 13),
        ];
        let encoded = encode_new_block_hashes(&request);
        let decoded = decode_new_block_hashes(&encoded).unwrap();
        assert_eq!(request, decoded);
    }

    #[test]
    fn test_encode_get_block_headers() {
        let request = GetBlockHeaders::new(BlockId::Number(1024), 128u64, 0u64, true);
        let encoded = encode_get_block_headers(&request);
        assert_eq!(encoded, [0xc7, 0x82, 0x04, 0x00, 0x81, 0x80, 0x80, 0x01]);
        let request = GetBlockHeaders::new(BlockId::Number(4096), 1u64, 10, false);
        let encoded = encode_get_block_headers(&request);
        assert_eq!(encoded, [0xc6, 0x82, 0x10, 0x00, 0x01, 0x0a, 0x80]);
    }

    #[test]
    fn test_decode_get_block_headers_with_hash_as_id() {
        let data: Vec<u8> = vec![
            228, 160, 229, 229, 95, 194, 152, 198, 135, 130, 236, 183, 27, 149, 246, 32, 35, 98,
            190, 1, 185, 199, 112, 109, 151, 50, 226, 8, 58, 130, 147, 155, 184, 73, 1, 128, 128,
        ];
        let expected_hash = BlockId::Hash(H256::from_slice(&data[2..34]));
        let expected = GetBlockHeaders::new(expected_hash, 1, 0, false);
        let decoded = decode_get_block_headers(&data).unwrap();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_get_block_headers_roundtrip() {
        let test_cases = vec![
            GetBlockHeaders::new(BlockId::Number(2283397), 100, 0, false),
            GetBlockHeaders::new(BlockId::Number(2700031), 1024, 8, true),
            GetBlockHeaders::new(BlockId::Hash(H256::repeat_byte(0x22)), 10, 1, false),
        ];
        for test_case in test_cases {
            let encoded = encode_get_block_headers(&test_case.clone());
            let decoded = decode_get_block_headers(&encoded).unwrap();
            assert_eq!(test_case, decoded);
        }
    }

    #[test]
    fn test_decode_block_header() {
        let header = vec![
            249, 2, 26, 249, 2, 23, 160, 150, 107, 246, 132, 157, 169, 47, 242, 160, 227, 219, 154,
            55, 31, 91, 159, 7, 221, 96, 1, 226, 119, 10, 66, 105, 165, 193, 52, 241, 191, 156, 76,
            160, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211,
            18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 148, 234, 103, 79,
            221, 231, 20, 253, 151, 157, 227, 237, 240, 245, 106, 169, 113, 107, 137, 142, 200,
            160, 116, 71, 126, 170, 190, 206, 107, 206, 0, 195, 70, 220, 18, 39, 91, 46, 215, 78,
            201, 214, 199, 88, 196, 2, 60, 32, 64, 186, 14, 114, 224, 93, 160, 20, 230, 203, 133,
            194, 42, 226, 253, 119, 79, 24, 204, 214, 103, 211, 254, 150, 125, 110, 57, 235, 197,
            34, 70, 131, 127, 35, 15, 2, 248, 69, 221, 160, 195, 99, 51, 64, 229, 167, 39, 232,
            170, 29, 41, 163, 175, 206, 149, 210, 126, 85, 90, 49, 167, 176, 151, 41, 103, 47, 55,
            108, 47, 63, 78, 46, 185, 1, 0, 136, 100, 128, 192, 2, 0, 98, 13, 132, 24, 13, 4, 112,
            0, 12, 80, 48, 129, 22, 0, 68, 208, 80, 21, 128, 128, 3, 116, 1, 16, 112, 96, 18, 0,
            64, 16, 82, 129, 16, 1, 0, 16, 69, 0, 65, 66, 3, 4, 10, 32, 128, 3, 72, 20, 32, 6, 16,
            218, 18, 8, 166, 56, 209, 110, 68, 12, 2, 72, 128, 128, 3, 1, 225, 0, 76, 43, 2, 40,
            80, 96, 32, 0, 8, 76, 50, 73, 160, 192, 132, 86, 156, 144, 194, 0, 32, 1, 88, 98, 65,
            4, 30, 128, 4, 3, 90, 68, 0, 160, 16, 9, 56, 0, 30, 4, 17, 128, 8, 49, 128, 176, 52, 6,
            97, 55, 32, 96, 64, 20, 40, 192, 32, 8, 116, 16, 64, 43, 148, 132, 2, 129, 0, 4, 148,
            129, 144, 12, 8, 3, 72, 100, 49, 70, 136, 208, 1, 84, 140, 48, 0, 130, 142, 84, 34,
            132, 24, 2, 128, 0, 100, 2, 162, 138, 2, 100, 218, 0, 172, 34, 48, 4, 0, 98, 9, 96,
            152, 50, 6, 96, 50, 0, 8, 64, 64, 18, 42, 71, 57, 8, 5, 1, 37, 21, 66, 8, 32, 32, 164,
            8, 124, 0, 2, 129, 192, 136, 0, 137, 141, 9, 0, 2, 64, 71, 56, 0, 0, 18, 112, 56, 9,
            142, 9, 8, 1, 8, 0, 0, 66, 144, 200, 66, 1, 102, 16, 64, 32, 2, 1, 192, 0, 75, 132,
            144, 173, 88, 136, 4, 135, 8, 121, 44, 111, 71, 247, 15, 131, 152, 150, 128, 131, 152,
            112, 92, 131, 152, 36, 179, 132, 94, 176, 23, 5, 150, 80, 80, 89, 69, 45, 101, 116,
            104, 101, 114, 109, 105, 110, 101, 45, 97, 115, 105, 97, 49, 45, 49, 160, 55, 253, 227,
            17, 117, 254, 24, 3, 70, 68, 77, 21, 180, 223, 198, 169, 218, 59, 43, 65, 238, 34, 152,
            206, 236, 202, 248, 136, 178, 212, 93, 244, 136, 47, 105, 35, 248, 4, 38, 241, 87,
        ];
        let decoded = decode_block_headers(&header);
        assert!(decoded.is_ok(), "Error: {}", decoded.err().unwrap());
    }

    #[test]
    fn test_block_body_roundtrip() {
        let tx = Transaction::default();
        let block_body = BlockBody {
            transactions: vec![tx.clone()],
            ommers: vec![],
        };
        let block_bodies = vec![block_body.clone()];
        let encoded = encode_block_bodies(&block_bodies);
        let decoded = decode_block_bodies(&encoded).unwrap();
        //assert_eq!(block_body, decoded[0]);
    }

    #[test]
    fn test_block_body_with_ommer_roundtrip() {
        let encoded = std::fs::read("src/block_manager/test_data/block_11_927_383").unwrap();
        let decoded = decode_block_bodies(&encoded).unwrap();
        let recovered = encode_block_bodies(&decoded);
        assert_eq!(encoded, recovered);
    }
}
