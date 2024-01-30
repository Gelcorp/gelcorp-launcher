use rsa::{ pkcs1::DecodeRsaPublicKey, RsaPublicKey };
use serde::Deserialize;

const PUBLIC_KEY: &str = include_str!("public_key.pem");
const AES_KEYS: &[u8] = include_bytes!("aes_key.json");

pub fn get_public_key() -> Result<RsaPublicKey, rsa::pkcs1::Error> {
  RsaPublicKey::from_pkcs1_pem(PUBLIC_KEY)
}

pub fn get_aes_keys() -> Result<AesKey, serde_json::Error> {
  serde_json::from_slice(AES_KEYS)
}

#[derive(Debug, Deserialize)]
pub struct AesKey {
  #[serde(with = "hex::serde")]
  key: [u8; 32],
  #[serde(with = "hex::serde")]
  iv: [u8; 16],
}

impl AesKey {
  pub fn key(&self) -> &[u8; 32] {
    &self.key
  }

  pub fn iv(&self) -> &[u8; 16] {
    &self.iv
  }
}
