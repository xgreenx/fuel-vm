use crate::common::Bytes32;

use digest::Digest;
use sha2::Sha256;

pub(crate) type Hash = Sha256;

pub fn zero_sum<V>() -> V
where
    V: From<Bytes32>,
{
    const ZERO_SUM: Bytes32 = [0; 32];
    ZERO_SUM.into()
}

pub fn sum<I, V>(data: I) -> V
where
    I: AsRef<[u8]>,
    V: From<Bytes32>,
{
    let mut hash = Hash::new();
    hash.update(data);
    let bytes: Bytes32 = hash.finalize().try_into().unwrap();
    bytes.into()
}

pub fn sum_all<I, V>(data: I) -> V
where
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
    V: From<Bytes32>,
{
    let mut hash = Hash::new();
    for datum in data.into_iter() {
        hash.update(datum)
    }
    let bytes: Bytes32 = hash.finalize().try_into().unwrap();
    bytes.into()
}
