use super::{H128, H160, H256, H512, U128, U256, U512};
use grpc_interfaces::types::{H128 as gH128, H160 as gH160, H256 as gH256, H512 as gH512};

pub fn from_grpc_h128(h: &gH128) -> H128 {
    let hash = [h.hi.to_be_bytes(), h.lo.to_be_bytes()].concat();
    H128::from_slice(&hash)
}

pub fn from_grpc_h160(h: gH160) -> H160 {
    let hash = [
        h.hi.as_ref().unwrap().hi.to_be_bytes().as_ref(),
        h.hi.as_ref().unwrap().lo.to_be_bytes().as_ref(),
        h.lo.to_be_bytes().to_owned().as_ref(),
    ]
    .concat();
    H160::from_slice(&hash)
}

pub fn from_grpc_h256(h: &gH256) -> H256 {
    let hash = [
        h.hi.as_ref().unwrap().hi.to_be_bytes(),
        h.hi.as_ref().unwrap().lo.to_be_bytes(),
        h.lo.as_ref().unwrap().hi.to_be_bytes(),
        h.lo.as_ref().unwrap().lo.to_be_bytes(),
    ]
    .concat();
    H256::from_slice(&hash)
}

pub fn from_grpc_h512(h: &gH512) -> H512 {
    let hash = [
        h.hi.as_ref().unwrap().hi.as_ref().unwrap().hi.to_be_bytes(),
        h.hi.as_ref().unwrap().hi.as_ref().unwrap().lo.to_be_bytes(),
        h.hi.as_ref().unwrap().lo.as_ref().unwrap().hi.to_be_bytes(),
        h.hi.as_ref().unwrap().lo.as_ref().unwrap().lo.to_be_bytes(),
        h.lo.as_ref().unwrap().hi.as_ref().unwrap().hi.to_be_bytes(),
        h.lo.as_ref().unwrap().hi.as_ref().unwrap().lo.to_be_bytes(),
        h.lo.as_ref().unwrap().lo.as_ref().unwrap().hi.to_be_bytes(),
        h.lo.as_ref().unwrap().lo.as_ref().unwrap().lo.to_be_bytes(),
    ]
    .concat();
    H512::from_slice(&hash)
}

pub fn to_grpc_h128(h: &H128) -> gH128 {
    let u = U128::from_big_endian(h.as_bytes());
    gH128 {
        hi: u.0[1],
        lo: u.0[0],
    }
}

// pub fn to_grpc_h160(h: &H160) -> gH160 {
//     let u = U160::from_big_endian(h.as_bytes());
//     gH160 {
//         hi: u.0[1],
//         lo: u.0[0],
//     }
// }

pub fn to_grpc_h256(h: &H256) -> gH256 {
    let u = U256::from_big_endian(h.as_bytes());
    gH256 {
        hi: Some(gH128 {
            hi: u.0[3],
            lo: u.0[2],
        }),
        lo: Some(gH128 {
            hi: u.0[1],
            lo: u.0[0],
        }),
    }
}

pub fn to_grpc_h512(h: &H512) -> gH512 {
    let u = U512::from_big_endian(h.as_bytes());
    gH512 {
        hi: Some(gH256 {
            hi: Some(gH128 {
                hi: u.0[7],
                lo: u.0[6],
            }),
            lo: Some(gH128 {
                hi: u.0[5],
                lo: u.0[4],
            }),
        }),
        lo: Some(gH256 {
            hi: Some(gH128 {
                hi: u.0[3],
                lo: u.0[2],
            }),
            lo: Some(gH128 {
                hi: u.0[1],
                lo: u.0[0],
            }),
        }),
    }
}
