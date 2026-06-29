use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Manager responsible for generating, validating, and tracking token lifecycles to prevent replay attacks.
pub struct TokenManager {
    secret: Vec<u8>,
    // Store seen signatures and their associated timestamps to enforce single-use.
    seen_signatures: Mutex<HashMap<String, u64>>,
}

impl TokenManager {
    /// Creates a new TokenManager with the specified HMAC secret.
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            secret,
            seen_signatures: Mutex::new(HashMap::new()),
        }
    }

    /// Generates a signed token for a given node_id.
    /// Returns a string formatted as "node_id:timestamp:signature".
    pub fn generate_token(&self, node_id: &str) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let payload = format!("{node_id}:{now}");
        let mut mac = HmacSha256::new_from_slice(&self.secret).map_err(|e| e.to_string())?;
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        Ok(format!("{node_id}:{now}:{signature}"))
    }

    /// Validates a token signature, verifies node ID, checks 60s expiration, and detects replay attacks.
    pub fn validate_token(&self, token: &str, expected_node_id: &str) -> Result<(), String> {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 3 {
            return Err("Malformed token format".to_string());
        }
        let node_id = parts[0];
        let timestamp_str = parts[1];
        let signature = parts[2];

        if node_id != expected_node_id {
            return Err("Token node_id mismatch".to_string());
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let timestamp = timestamp_str.parse::<u64>().map_err(|e| e.to_string())?;

        // Expiration check: tokens expire after 60 seconds
        if now < timestamp || now > timestamp + 60 {
            return Err("Token expired or invalid timestamp".to_string());
        }

        // Validate signature
        let payload = format!("{node_id}:{timestamp_str}");
        let mut mac = HmacSha256::new_from_slice(&self.secret).map_err(|e| e.to_string())?;
        mac.update(payload.as_bytes());
        let expected_signature = hex::encode(mac.finalize().into_bytes());

        if signature != expected_signature {
            return Err("Invalid token signature".to_string());
        }

        // Lock map to check for replays and perform garbage collection
        let mut seen = self.seen_signatures.lock().map_err(|e| e.to_string())?;
        if seen.contains_key(signature) {
            return Err("Replayed token detected".to_string());
        }

        // Insert new token signature
        seen.insert(signature.to_string(), timestamp);

        // Prune old entries to prevent memory leaks (older than 60s)
        let cutoff = now.saturating_sub(60);
        seen.retain(|_, ts| *ts >= cutoff);

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_token_lifecycle() {
        let secret = b"supersecretkeyforauthsupersecretkeyforauth".to_vec();
        let manager = TokenManager::new(secret);
        let node_id = "blade-01";

        // Generate token
        let token = manager.generate_token(node_id).unwrap();

        // Validate token
        assert!(manager.validate_token(&token, node_id).is_ok());

        // Replay validation must fail
        assert_eq!(
            manager.validate_token(&token, node_id).unwrap_err(),
            "Replayed token detected"
        );
    }

    #[test]
    fn test_token_mismatched_node() {
        let secret = b"supersecretkeyforauthsupersecretkeyforauth".to_vec();
        let manager = TokenManager::new(secret);
        let token = manager.generate_token("blade-01").unwrap();

        assert_eq!(
            manager.validate_token(&token, "blade-02").unwrap_err(),
            "Token node_id mismatch"
        );
    }

    #[test]
    fn test_token_expired() {
        let secret = b"supersecretkeyforauthsupersecretkeyforauth".to_vec();
        let manager = TokenManager::new(secret);
        let node_id = "blade-01";

        // Generate a token with a timestamp in the past (e.g. 100 seconds ago)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let past_time = now - 100;
        let payload = format!("{node_id}:{past_time}");

        let mut mac = HmacSha256::new_from_slice(&manager.secret).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let expired_token = format!("{node_id}:{past_time}:{signature}");

        assert_eq!(
            manager.validate_token(&expired_token, node_id).unwrap_err(),
            "Token expired or invalid timestamp"
        );
    }

    #[test]
    fn test_token_invalid_signature() {
        let secret = b"supersecretkeyforauthsupersecretkeyforauth".to_vec();
        let manager = TokenManager::new(secret);
        let node_id = "blade-01";
        let token = manager.generate_token(node_id).unwrap();
        let mut parts: Vec<&str> = token.split(':').collect();
        parts[2] = "invalid_signature_hex_code_123456";
        let tampered_token = format!("{}:{}:{}", parts[0], parts[1], parts[2]);

        assert!(manager.validate_token(&tampered_token, node_id).is_err());
    }
}
