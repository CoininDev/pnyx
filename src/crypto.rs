use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, SignatureError};
use primitive_types::H256;
use sha2::{Sha256, Digest};
use rand::Rng;

/// Módulo de criptografia para Pnyx
/// Responsável por geração de chaves, assinatura e verificação

#[derive(Clone, Debug)]
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Gera um novo key pair para um nó
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Deriva a chave de um nó a partir da chave do maintainer
    /// Seguindo: node_key = HKDF(maintainer_privkey, node_id)
    pub fn derive_node_key(maintainer_key: &KeyPair, node_id: u8) -> Self {
        use sha2::Sha256;

        let maintainer_bytes = maintainer_key.signing_key.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&maintainer_bytes);
        hasher.update(&[node_id]);
        let hash = hasher.finalize();

        // Usa os 32 bytes do hash como seed para a chave do nó
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&hash[..32]);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Retorna o endereço (H256) da chave pública
    pub fn address(&self) -> H256 {
        let public_bytes = self.verifying_key.to_bytes();
        let mut hash = Sha256::new();
        hash.update(&public_bytes);
        let result = hash.finalize();
        H256::from_slice(&result[..32])
    }

    /// Assina uma mensagem
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Retorna a chave pública em bytes
    pub fn public_bytes(&self) -> Vec<u8> {
        self.verifying_key.to_bytes().to_vec()
    }
}

/// Verifica assinatura de uma mensagem
pub fn verify_signature(
    message: &[u8],
    signature: &[u8],
    public_key_bytes: &[u8],
) -> Result<(), SignatureError> {
    // Converter bytes para array de 32 bytes
    let pub_key_array: [u8; 32] = public_key_bytes.try_into()
        .map_err(|_| SignatureError::new())?;
    
    let verifying_key = VerifyingKey::from_bytes(&pub_key_array)?;

    let sig_array: [u8; 64] = signature.try_into()
        .map_err(|_| SignatureError::new())?;
    
    let sig = Signature::from_bytes(&sig_array);

    verifying_key.verify_strict(message, &sig)
}

/// Calcula Keccak-256 hash
pub fn keccak256(data: &[u8]) -> H256 {
    use tiny_keccak::{Hasher, Keccak};
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    H256::from_slice(&hash)
}

/// Calcula SHA256 hash
pub fn sha256(data: &[u8]) -> H256 {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    H256::from_slice(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let key = KeyPair::generate();
        assert_eq!(key.verifying_key.to_bytes().len(), 32);
    }

    #[test]
    fn test_node_key_derivation() {
        let maintainer_key = KeyPair::generate();
        let node_key_1 = KeyPair::derive_node_key(&maintainer_key, 1);
        let node_key_2 = KeyPair::derive_node_key(&maintainer_key, 2);

        // Chaves derivadas devem ser diferentes
        assert_ne!(
            node_key_1.public_bytes(),
            node_key_2.public_bytes()
        );

        // Mesma derivação deve produzir mesma chave
        let node_key_1_again = KeyPair::derive_node_key(&maintainer_key, 1);
        assert_eq!(
            node_key_1.public_bytes(),
            node_key_1_again.public_bytes()
        );
    }

    #[test]
    fn test_signing_and_verification() {
        let key = KeyPair::generate();
        let message = b"test message";

        let signature = key.sign(message);
        assert!(verify_signature(message, &signature, &key.public_bytes()).is_ok());

        // Verificação com mensagem errada deve falhar
        let wrong_message = b"wrong message";
        assert!(verify_signature(wrong_message, &signature, &key.public_bytes()).is_err());
    }

    #[test]
    fn test_address_generation() {
        let key = KeyPair::generate();
        let address = key.address();
        assert_eq!(address.as_bytes().len(), 32);
    }

    #[test]
    fn test_keccak256() {
        let data = b"test";
        let hash = keccak256(data);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_sha256() {
        let data = b"test";
        let hash = sha256(data);
        assert_eq!(hash.as_bytes().len(), 32);
    }
}
