use anyhow::{Context, Result};
use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use std::str::FromStr;

/// Wallet manager for loading and managing keypairs
#[derive(Clone)]
pub struct Wallet {
    keypair: Keypair,
}

impl Wallet {
    /// Create wallet from base58-encoded private key
    pub fn from_base58(private_key: &str) -> Result<Self> {
        let bytes = bs58::decode(private_key)
            .into_vec()
            .context("Failed to decode base58 private key")?;

        let keypair = Keypair::from_bytes(&bytes)
            .context("Failed to create keypair from bytes")?;

        Ok(Self { keypair })
    }

    /// Create wallet from mnemonic phrase
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self> {
        let seed = bip39::Mnemonic::from_str(mnemonic)
            .context("Failed to parse mnemonic")?
            .to_seed("");

        // Use first 32 bytes as seed for keypair
        let seed_bytes: [u8; 32] = seed[..32]
            .try_into()
            .context("Failed to extract seed bytes")?;

        let keypair = Keypair::from_bytes(&seed_bytes)
            .context("Failed to create keypair from seed")?;

        Ok(Self { keypair })
    }

    /// Get the public key (wallet address)
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get a reference to the keypair for signing
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Load wallet from config (tries private_key first, then mnemonic)
    pub fn from_config(config: &crate::config::Config) -> Result<Self> {
        if let Some(ref private_key) = config.private_key {
            Self::from_base58(private_key)
                .context("Failed to load wallet from private key")
        } else if let Some(ref mnemonic) = config.mnemonic {
            Self::from_mnemonic(mnemonic)
                .context("Failed to load wallet from mnemonic")
        } else {
            anyhow::bail!(
                "No wallet credentials found. Set PRIVATE_KEY_BASE58 or MNEMONIC environment variable"
            )
        }
    }
}
