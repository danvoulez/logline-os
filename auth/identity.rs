//! LogLine ID - Core Identity Service
//!
//! This module provides the core identity system for LogLine,
//! designed to work as an independent microservice.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::de::Error;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Conveniência para compatibilidade com o código existente.
pub type LogLineIDWithKeys = LogLineKeyPair;

/// Core LogLine Identity Structure
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LogLineID {
    pub id: Uuid,
    pub node_name: String,
    pub public_key: String,
    pub alias: Option<String>,
    pub tenant_id: Option<String>,
    pub is_org: bool,
    pub metadata: Option<serde_json::Value>,
    pub issued_at: DateTime<Utc>,
}

/// Par de chaves LogLine que inclui a chave privada e o ID
#[derive(Clone)]
pub struct LogLineKeyPair {
    pub signing_key: SigningKey,
    pub id: LogLineID,
}

impl Serialize for LogLineKeyPair {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("LogLineKeyPair", 2)?;
        state.serialize_field(
            "signing_key",
            &URL_SAFE_NO_PAD.encode(self.signing_key.to_bytes()),
        )?;
        state.serialize_field("id", &self.id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for LogLineKeyPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            signing_key: String,
            id: LogLineID,
        }

        let helper = Helper::deserialize(deserializer)?;
        let key_bytes = URL_SAFE_NO_PAD
            .decode(helper.signing_key.as_bytes())
            .map_err(D::Error::custom)?;

        if key_bytes.len() != 32 {
            return Err(D::Error::custom("chave de assinatura inválida"));
        }

        let mut secret_key = [0u8; 32];
        secret_key.copy_from_slice(&key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_key);
        let verifying_key = signing_key.verifying_key();

        // Ensure stored public key matches the signing key
        let expected_pk = URL_SAFE_NO_PAD
            .decode(helper.id.public_key.as_bytes())
            .map_err(D::Error::custom)?;
        if verifying_key.as_bytes() != expected_pk.as_slice() {
            return Err(D::Error::custom(
                "chave pública não corresponde ao LogLineID fornecido",
            ));
        }

        Ok(Self {
            signing_key,
            id: helper.id,
        })
    }
}

impl LogLineID {
    pub fn new(
        node_name: &str,
        public_key: &VerifyingKey,
        alias: Option<String>,
        tenant_id: Option<String>,
        is_org: bool,
    ) -> Self {
        let pk_encoded = URL_SAFE_NO_PAD.encode(public_key.as_bytes());

        Self {
            id: Uuid::new_v4(),
            node_name: node_name.to_string(),
            public_key: pk_encoded,
            alias,
            tenant_id,
            is_org,
            metadata: None,
            issued_at: Utc::now(),
        }
    }

    pub fn with_alias(mut self, alias: &str) -> Self {
        self.alias = Some(alias.to_string());
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_is_org(mut self, is_org: bool) -> Self {
        self.is_org = is_org;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn get_public_key(&self) -> Result<VerifyingKey, String> {
        let pk_bytes = URL_SAFE_NO_PAD
            .decode(&self.public_key)
            .map_err(|e| format!("Erro ao decodificar a chave pública: {}", e))?;

        let pk_array: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| "Chave pública inválida: tamanho incorreto".to_string())?;

        VerifyingKey::from_bytes(&pk_array).map_err(|e| format!("Chave pública inválida: {}", e))
    }

    pub fn verify_signature(&self, message: &[u8], signature: &[u8]) -> Result<bool, String> {
        let public_key = self.get_public_key()?;

        if signature.len() != 64 {
            return Err("Tamanho de assinatura inválido".to_string());
        }

        let signature_array: [u8; 64] = signature
            .try_into()
            .map_err(|_| "Erro ao converter assinatura".to_string())?;
        let signature = Signature::from_bytes(&signature_array);

        Ok(public_key.verify(message, &signature).is_ok())
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn display_name(&self) -> String {
        if let Some(alias) = &self.alias {
            if self.is_org {
                format!("Organização: {}", alias)
            } else {
                alias.clone()
            }
        } else {
            let short_key = if self.public_key.len() > 16 {
                format!("{}...", &self.public_key[..16])
            } else {
                self.public_key.clone()
            };

            if self.is_org {
                format!("Org: {}", short_key)
            } else {
                format!("ID: {}", short_key)
            }
        }
    }

    pub fn from_string(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn sign(&self, signing_key: &SigningKey, message: &[u8]) -> Signature {
        signing_key.sign(message)
    }

    pub fn save_to_file(&self, signing_key: &[u8]) -> Result<(), String> {
        let home_dir = dirs::home_dir().ok_or("Não foi possível obter o diretório home")?;
        let logline_dir = home_dir.join(".logline");

        if !logline_dir.exists() {
            std::fs::create_dir_all(&logline_dir)
                .map_err(|e| format!("Erro ao criar diretório ~/.logline: {}", e))?;
        }

        let file_name = self.alias.as_deref().unwrap_or(&self.node_name);
        let file_path = logline_dir.join(file_name);

        let json = self
            .to_json()
            .map_err(|e| format!("Erro ao serializar ID: {}", e))?;

        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), serde_json::from_str(&json).unwrap());
        obj.insert(
            "signing_key".to_string(),
            serde_json::Value::String(URL_SAFE_NO_PAD.encode(signing_key)),
        );

        let json_with_key =
            serde_json::to_string_pretty(&obj).map_err(|e| format!("Erro ao serializar: {}", e))?;

        std::fs::write(&file_path, json_with_key)
            .map_err(|e| format!("Erro ao escrever arquivo {}: {}", file_path.display(), e))?;

        Ok(())
    }

    pub fn load_from_file(alias: &str) -> Result<LogLineKeyPair, String> {
        let home_dir = dirs::home_dir().ok_or("Não foi possível obter o diretório home")?;
        let file_path = home_dir.join(".logline").join(alias);

        if !file_path.exists() {
            return Err(format!("Arquivo não encontrado: {}", file_path.display()));
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Erro ao ler arquivo {}: {}", file_path.display(), e))?;

        let data: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("Erro ao deserializar: {}", e))?;

        let id_value = data.get("id").ok_or("ID não encontrado no arquivo")?;

        let id: LogLineID = serde_json::from_value(id_value.clone())
            .map_err(|e| format!("Erro ao deserializar ID: {}", e))?;

        let key_str = data
            .get("signing_key")
            .and_then(|v| v.as_str())
            .ok_or("Chave de assinatura não encontrada")?;

        let key_bytes = URL_SAFE_NO_PAD
            .decode(key_str)
            .map_err(|e| format!("Erro ao decodificar chave: {}", e))?;

        LogLineKeyPair::from_secret_key(
            &id.node_name,
            &key_bytes,
            id.alias.clone(),
            id.tenant_id.clone(),
            id.is_org,
        )
        .map(|mut keypair| {
            keypair.id.id = id.id;
            keypair.id.metadata = id.metadata.clone();
            keypair.id.issued_at = id.issued_at;
            keypair
        })
    }
}

impl fmt::Display for LogLineID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for LogLineID {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl LogLineKeyPair {
    pub fn generate(
        node_name: &str,
        alias: Option<String>,
        tenant_id: Option<String>,
        is_org: bool,
    ) -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let id = LogLineID::new(node_name, &verifying_key, alias, tenant_id, is_org);

        Self { signing_key, id }
    }

    pub fn from_secret_key(
        node_name: &str,
        secret_key_bytes: &[u8],
        alias: Option<String>,
        tenant_id: Option<String>,
        is_org: bool,
    ) -> Result<Self, String> {
        if secret_key_bytes.len() != 32 {
            return Err("Tamanho inválido para chave secreta".to_string());
        }

        let mut secret_array = [0u8; 32];
        secret_array.copy_from_slice(secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_array);
        let verifying_key = signing_key.verifying_key();
        let id = LogLineID::new(node_name, &verifying_key, alias, tenant_id, is_org);

        Ok(Self { signing_key, id })
    }

    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn export_secret_key(&self, _password: &str) -> Result<String, String> {
        Ok(URL_SAFE_NO_PAD.encode(self.signing_key.to_bytes()))
    }

    pub fn import_secret_key(
        node_name: &str,
        encoded: &str,
        _password: &str,
        alias: Option<String>,
        tenant_id: Option<String>,
        is_org: bool,
    ) -> Result<Self, String> {
        let key_bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| format!("Erro ao decodificar a chave: {}", e))?;

        Self::from_secret_key(node_name, &key_bytes, alias, tenant_id, is_org)
    }
}

/// Facilita criação de LogLine IDs e KeyPairs
pub struct LogLineIDBuilder;

impl LogLineID {
    pub fn generate(node_name: &str) -> LogLineKeyPair {
        LogLineIDBuilder::new_user(node_name, Some(node_name.to_string()), None)
    }
}

impl LogLineIDBuilder {
    pub fn new_user(
        node_name: &str,
        alias: Option<String>,
        tenant_id: Option<String>,
    ) -> LogLineKeyPair {
        LogLineKeyPair::generate(node_name, alias, tenant_id, false)
    }

    pub fn new_organization(
        node_name: &str,
        alias: Option<String>,
        tenant_id: Option<String>,
    ) -> LogLineKeyPair {
        LogLineKeyPair::generate(node_name, alias, tenant_id, true)
    }

    pub fn new_system(node_name: &str) -> LogLineKeyPair {
        let mut keypair =
            LogLineKeyPair::generate(node_name, Some("LogLine System".to_string()), None, true);

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "type".to_string(),
            serde_json::Value::String("system".to_string()),
        );
        metadata.insert(
            "permissions".to_string(),
            serde_json::Value::String("all".to_string()),
        );

        keypair.id = keypair
            .id
            .with_metadata(serde_json::Value::Object(metadata));
        keypair
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = LogLineKeyPair::generate(
            "test-node",
            Some("Test User".to_string()),
            Some("tenant1".to_string()),
            false,
        );
        assert_eq!(keypair.id.node_name, "test-node");
        assert_eq!(keypair.id.alias, Some("Test User".to_string()));
        assert_eq!(keypair.id.tenant_id, Some("tenant1".to_string()));
        assert!(!keypair.id.is_org);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = LogLineKeyPair::generate("test-node", None, None, false);
        let message = b"Hello, LogLine!";

        let signature = keypair.id.sign(&keypair.signing_key, message);
        assert!(keypair
            .id
            .verify_signature(message, &signature.to_bytes())
            .unwrap());

        let wrong_message = b"Wrong message!";
        assert!(!keypair
            .id
            .verify_signature(wrong_message, &signature.to_bytes())
            .unwrap());
    }

    #[test]
    fn test_serialization() {
        let keypair = LogLineIDBuilder::new_user(
            "test-node",
            Some("Usuário de Teste".to_string()),
            Some("tenant99".to_string()),
        );

        let json = serde_json::to_string(&keypair).unwrap();
        let deserialized: LogLineKeyPair = serde_json::from_str(&json).unwrap();

        assert_eq!(keypair.id.public_key, deserialized.id.public_key);
        assert_eq!(keypair.id.alias, deserialized.id.alias);
        assert_eq!(keypair.id.tenant_id, deserialized.id.tenant_id);
        assert_eq!(keypair.id.is_org, deserialized.id.is_org);
        assert_eq!(keypair.id.node_name, deserialized.id.node_name);
    }

    #[test]
    fn test_display_name() {
        let id1 = LogLineIDBuilder::new_user("alice-node", Some("Alice".to_string()), None).id;
        let id2 =
            LogLineIDBuilder::new_organization("acme-node", Some("ACME Corp".to_string()), None).id;
        let id3 = LogLineIDBuilder::new_user("anon-node", None, None).id;

        assert_eq!(id1.display_name(), "Alice");
        assert_eq!(id2.display_name(), "Organização: ACME Corp");
        assert!(id3.display_name().starts_with("ID: "));
    }
}
