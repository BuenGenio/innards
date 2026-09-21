//! Offline license keys. A key is `INNARDS-<payload>.<signature>` where both
//! parts are base64url (no padding); the payload is JSON and the signature
//! is Ed25519 over the raw payload bytes, made with the private key that
//! only the `innards-license` tool holds. No server, no phone-home.
//!
//! Tiers: `free` (default), `supporter` (one-time), `pro` (yearly, has `exp`).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Public half of the keypair from `innards-license keygen`. The private
/// half lives only in ~/.config/innards/signing.key on the issuing machine.
pub const PUBLIC_KEY_B64: &str = "SumRSoonqLVNgT53FcqQzr4FHjjJP69uGCmHZNJua8s";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Free,
    Supporter,
    Pro,
    /// Team and Enterprise keys carry an `org`; the app can then upload
    /// reports to the org's Threadwise workspace.
    Team,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub tier: Tier,
    pub email: String,
    /// ISO date; Pro/Team/Enterprise keys expire, Supporter keys don't.
    #[serde(default)]
    pub exp: Option<String>,
    /// Organisation slug for Team/Enterprise keys.
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub iat: Option<String>,
    /// Stripe subscription / customer ids (Pro, Team); only used to ask the
    /// billing service for a renewed key.
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub cust: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub tier: Tier,
    pub email: Option<String>,
    pub expires: Option<String>,
    pub org: Option<String>,
    pub source: String,
    /// True for subscription keys that can be refreshed via the billing service.
    pub renewable: bool,
}

pub fn status(key: Option<&str>) -> Status {
    // Developer override so the paid UI can be exercised without a key.
    if let Ok(t) = std::env::var("INNARDS_TIER") {
        let tier = match t.as_str() {
            "enterprise" => Tier::Enterprise,
            "team" => Tier::Team,
            "pro" => Tier::Pro,
            "supporter" => Tier::Supporter,
            _ => Tier::Free,
        };
        return Status { tier, email: None, expires: None, org: std::env::var("INNARDS_ORG").ok(), source: "env".into(), renewable: false };
    }
    match key.and_then(verify) {
        Some(p) => Status { tier: p.tier, email: Some(p.email), expires: p.exp, org: p.org, source: "key".into(), renewable: p.sub.is_some() },
        None => Status { tier: Tier::Free, email: None, expires: None, org: None, source: "none".into(), renewable: false },
    }
}

pub fn verify(key: &str) -> Option<Payload> {
    let rest = key.trim().strip_prefix("INNARDS-")?;
    let (payload_b64, sig_b64) = rest.split_once('.')?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let sig = URL_SAFE_NO_PAD.decode(sig_b64).ok()?;
    let pk_bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(PUBLIC_KEY_B64).ok()?.try_into().ok()?;
    let pk = VerifyingKey::from_bytes(&pk_bytes).ok()?;
    let sig = Signature::from_slice(&sig).ok()?;
    pk.verify(&payload, &sig).ok()?;
    let p: Payload = serde_json::from_slice(&payload).ok()?;
    if let Some(exp) = &p.exp {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if exp.as_str() < today.as_str() {
            return None;
        }
    }
    Some(p)
}

/// Base URL of the billing service (site + worker). Override for staging.
pub fn billing_url() -> String {
    std::env::var("INNARDS_BILLING_URL").unwrap_or_else(|_| "https://innards.app".into())
}

/// Ask the billing service for a renewed key. Only meaningful for
/// subscription keys; returns the new key when the subscription is active.
pub async fn refresh(key: &str) -> Result<String, String> {
    let url = format!("{}/api/license/refresh?key={}", billing_url(), urlencoding(key));
    let resp = reqwest::Client::new().get(&url).send().await.map_err(|e| format!("network: {e}"))?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(v.pointer("/error/code").and_then(|c| c.as_str()).unwrap_or("refresh_failed").to_string());
    }
    v.get("key").and_then(|k| k.as_str()).map(String::from).ok_or_else(|| "no key in response".into())
}

fn urlencoding(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}
