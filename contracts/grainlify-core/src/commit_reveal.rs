use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commitment {
    pub hash: BytesN<32>,
    pub creator: Address,
    pub timestamp: u64,
    pub expiry: Option<u64>,
}

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    CommitmentExpired = 100,
    RevealMismatch = 101,
}

/// Creates a new commitment.
///
/// # Arguments
/// * `env` - The environment
/// * `creator` - The address creating the commitment
/// * `hash` - The sha256 hash of (value + salt)
/// * `expiry` - Optional expiration timestamp
pub fn create_commitment(
    env: &Env,
    creator: Address,
    hash: BytesN<32>,
    expiry: Option<u64>,
) -> Commitment {
    Commitment {
        hash,
        creator,
        timestamp: env.ledger().timestamp(),
        expiry,
    }
}

/// Verifies a reveal against a commitment.
///
/// # Arguments
/// * `env` - The environment
/// * `commitment` - The stored commitment
/// * `value` - The revealed value (as Bytes)
/// * `salt` - The revealed salt (as Bytes)
///
/// # Returns
/// * `Result<(), Error>` - Ok if reveal matches, error otherwise
pub fn verify_reveal(
    env: &Env,
    commitment: &Commitment,
    value: Bytes,
    salt: Bytes,
) -> Result<(), Error> {
    // Check expiry
    if let Some(expiry) = commitment.expiry {
        if env.ledger().timestamp() > expiry {
            return Err(Error::CommitmentExpired);
        }
    }

    // Reconstruct hash: sha256(value + salt)
    let mut data = value;
    data.append(&salt);
    let reconstructed_hash: BytesN<32> = env.crypto().sha256(&data).into();

    if reconstructed_hash != commitment.hash {
        return Err(Error::RevealMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_commit_reveal_success() {
        let env = Env::default();
        let creator = Address::generate(&env);

        let value = Bytes::from_array(&env, &[1, 2, 3]);
        let salt = Bytes::from_array(&env, &[4, 5, 6]);

        // Prepare hash
        let mut data = value.clone();
        data.append(&salt);
        let hash: BytesN<32> = env.crypto().sha256(&data).into();

        let commitment = create_commitment(&env, creator.clone(), hash, None);

        let result = verify_reveal(&env, &commitment, value, salt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_reveal_mismatch() {
        let env = Env::default();
        let creator = Address::generate(&env);

        let value = Bytes::from_array(&env, &[1, 2, 3]);
        let salt = Bytes::from_array(&env, &[4, 5, 6]);

        let mut data = value.clone();
        data.append(&salt);
        let hash: BytesN<32> = env.crypto().sha256(&data).into();

        let commitment = create_commitment(&env, creator.clone(), hash, None);

        // Try reveal with wrong value
        let wrong_value = Bytes::from_array(&env, &[9, 9, 9]);
        let result = verify_reveal(&env, &commitment, wrong_value, salt);
        assert_eq!(result, Err(Error::RevealMismatch));
    }

    #[test]
    fn test_commit_reveal_expired() {
        let env = Env::default();
        env.ledger().with_mut(|li| li.timestamp = 1000);
        let creator = Address::generate(&env);

        let value = Bytes::from_array(&env, &[1, 2, 3]);
        let salt = Bytes::from_array(&env, &[4, 5, 6]);

        let mut data = value.clone();
        data.append(&salt);
        let hash: BytesN<32> = env.crypto().sha256(&data).into();

        // Expire at 1100
        let commitment = create_commitment(&env, creator.clone(), hash, Some(1100));

        // Fast forward to 1200
        env.ledger().with_mut(|li| li.timestamp = 1200);

        let result = verify_reveal(&env, &commitment, value, salt);
        assert_eq!(result, Err(Error::CommitmentExpired));
    }
}
