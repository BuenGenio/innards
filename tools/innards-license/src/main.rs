//! innards-license keygen                      -> writes ~/.config/innards/signing.key, prints public key
//! innards-license sign <tier> <email> [days] [org]  -> prints a license key
//!   tiers: supporter (no expiry) | pro | team | enterprise (org required)
//! innards-license verify <key>                -> checks a key against the local public key
//!
//! Key format matches src-tauri/src/license.rs: `INNARDS-<b64url(json)>.<b64url(sig)>`.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use std::path::PathBuf;

fn key_path() -> PathBuf {
    let dir = dirs::config_dir().expect("config dir").join("innards");
    std::fs::create_dir_all(&dir).expect("create config dir");
    dir.join("signing.key")
}

fn load_signing_key() -> SigningKey {
    let b64 = std::fs::read_to_string(key_path()).expect("no signing key; run `innards-license keygen` first");
    let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(b64.trim()).expect("bad key file").try_into().expect("bad key length");
    SigningKey::from_bytes(&bytes)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("keygen") => {
            let p = key_path();
            if p.exists() {
                eprintln!("refusing to overwrite existing key at {}", p.display());
                std::process::exit(1);
            }
            let sk = SigningKey::generate(&mut rand::rngs::OsRng);
            std::fs::write(&p, URL_SAFE_NO_PAD.encode(sk.to_bytes())).expect("write key");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).ok();
            }
            println!("private key written to {}", p.display());
            println!("public key (paste into src-tauri/src/license.rs PUBLIC_KEY_B64):");
            println!("{}", URL_SAFE_NO_PAD.encode(sk.verifying_key().to_bytes()));
        }
        Some("sign") => {
            let tier = args.get(1).expect("tier: supporter|pro");
            let email = args.get(2).expect("email");
            let days: Option<i64> = args.get(3).and_then(|d| d.parse().ok());
            let org = args.get(4);
            assert!(matches!(tier.as_str(), "supporter" | "pro" | "team" | "enterprise"), "tier must be supporter, pro, team or enterprise");
            if matches!(tier.as_str(), "team" | "enterprise") {
                assert!(org.is_some(), "team/enterprise keys need an org slug: sign team <email> <days> <org>");
            }
            let today = chrono::Utc::now();
            let mut payload = serde_json::json!({ "tier": tier, "email": email, "iat": today.format("%Y-%m-%d").to_string() });
            if let Some(o) = org {
                payload["org"] = serde_json::Value::String(o.clone());
            }
            if let Some(d) = days.or(if tier != "supporter" { Some(366) } else { None }) {
                payload["exp"] = serde_json::Value::String((today + chrono::Duration::days(d)).format("%Y-%m-%d").to_string());
            }
            let bytes = serde_json::to_vec(&payload).unwrap();
            let sig = load_signing_key().sign(&bytes);
            println!("INNARDS-{}.{}", URL_SAFE_NO_PAD.encode(&bytes), URL_SAFE_NO_PAD.encode(sig.to_bytes()));
        }
        Some("verify") => {
            let key = args.get(1).expect("key");
            let vk: VerifyingKey = load_signing_key().verifying_key();
            let parsed = (|| {
                let rest = key.trim().strip_prefix("INNARDS-")?;
                let (p, s) = rest.split_once('.')?;
                let payload = URL_SAFE_NO_PAD.decode(p).ok()?;
                let sig = ed25519_dalek::Signature::from_slice(&URL_SAFE_NO_PAD.decode(s).ok()?).ok()?;
                vk.verify(&payload, &sig).ok()?;
                Some(payload)
            })();
            match parsed {
                Some(payload) => println!("valid: {}", String::from_utf8_lossy(&payload)),
                None => {
                    println!("INVALID");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("usage: innards-license keygen | sign <supporter|pro|team|enterprise> <email> [days] [org] | verify <key>");
            std::process::exit(2);
        }
    }
}
