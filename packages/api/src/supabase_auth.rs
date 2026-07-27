//! Supabase JWT verification and authentication system
//! Ported from the previous backend system

use crate::domain::UserInfo;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::{decode_header, Algorithm, DecodingKey, Validation};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kty: String, // "EC" or "RSA"
    kid: String,
    alg: Option<String>, // "ES256" or "RS256"
    // EC fields
    crv: Option<String>, // "P-256"
    x: Option<String>,   // base64url
    y: Option<String>,   // base64url
    // RSA fields
    n: Option<String>, // base64url
    e: Option<String>, // base64url
    #[allow(dead_code)]
    use_: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

/// Simple in-memory JWKS cache with TTL
struct JwksCacheInner {
    jwks: Option<Jwks>,
    fetched_at: Option<Instant>,
}

pub struct JwksCache {
    inner: RwLock<JwksCacheInner>,
    ttl: Duration,
    url: String,
}

impl JwksCache {
    pub fn new(url: String, ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(JwksCacheInner {
                jwks: None,
                fetched_at: None,
            }),
            ttl,
            url,
        }
    }

    pub async fn get(&self) -> anyhow::Result<Jwks> {
        {
            let guard = self.inner.read().await;
            if let (Some(jwks), Some(t)) = (&guard.jwks, guard.fetched_at) {
                if t.elapsed() < self.ttl {
                    return Ok(jwks.clone());
                }
            }
        }
        let jwks = reqwest::Client::new()
            .get(&self.url)
            .send()
            .await?
            .json::<Jwks>()
            .await?;
        let mut guard = self.inner.write().await;
        guard.jwks = Some(jwks.clone());
        guard.fetched_at = Some(Instant::now());
        Ok(jwks)
    }

    /// Force re-fetch (used if a kid wasn't found)
    pub async fn refresh(&self) -> anyhow::Result<Jwks> {
        let jwks = reqwest::Client::new()
            .get(&self.url)
            .send()
            .await?
            .json::<Jwks>()
            .await?;
        let mut guard = self.inner.write().await;
        guard.jwks = Some(jwks.clone());
        guard.fetched_at = Some(Instant::now());
        Ok(jwks)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SupabaseClaims {
    pub sub: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
    pub iat: Option<usize>,
    pub iss: Option<String>,
    pub aud: Option<String>,
    // Add any other fields you rely on
}

/// Build a DecodingKey from EC JWK
fn decoding_key_from_ec_jwk(jwk: &Jwk) -> anyhow::Result<DecodingKey> {
    let x = jwk
        .x
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing x"))?;
    let y = jwk
        .y
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing y"))?;
    let crv = jwk
        .crv
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing crv"))?;
    anyhow::ensure!(jwk.kty == "EC" && crv == "P-256", "wrong key type/crv");
    Ok(DecodingKey::from_ec_components(x, y)?)
}

/// Build a DecodingKey from RSA JWK
fn decoding_key_from_rsa_jwk(jwk: &Jwk) -> anyhow::Result<DecodingKey> {
    let n = jwk
        .n
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing n"))?;
    let e = jwk
        .e
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing e"))?;
    anyhow::ensure!(jwk.kty == "RSA", "wrong key type");
    Ok(DecodingKey::from_rsa_components(n, e)?)
}

pub struct SupabaseVerifier {
    cache: Arc<JwksCache>,
    expected_iss: String,
    expected_aud: String,
}

impl SupabaseVerifier {
    pub fn new(project_ref: &str, expected_aud: &str) -> Self {
        let jwks_url = format!(
            "https://{}.supabase.co/auth/v1/.well-known/jwks.json",
            project_ref
        );
        Self {
            cache: Arc::new(JwksCache::new(jwks_url, Duration::from_secs(60 * 10))), // 10 min
            expected_iss: format!("https://{}.supabase.co/auth/v1", project_ref),
            expected_aud: expected_aud.to_string(),
        }
    }

    pub async fn verify(&self, token: &str) -> anyhow::Result<UserInfo> {
        let header = decode_header(token)?;
        let alg = header.alg;
        let kid = header.kid.ok_or_else(|| anyhow::anyhow!("missing kid"))?;

        // Strict alg allow-list; allow ES256 (and RS256 if you want dual support)
        match alg {
            Algorithm::ES256 | Algorithm::RS256 => {}
            other => anyhow::bail!("unexpected alg: {:?}", other),
        }

        // Pull JWKS (cache + refresh on miss)
        let mut jwks = self.cache.get().await?;
        let mut jwk = jwks.keys.iter().find(|k| k.kid == kid);
        if jwk.is_none() {
            jwks = self.cache.refresh().await?;
            jwk = jwks.keys.iter().find(|k| k.kid == kid);
        }
        let jwk = jwk.ok_or_else(|| anyhow::anyhow!("kid not found in JWKS"))?;

        // Build DecodingKey depending on alg/key type
        let decoding_key = match alg {
            Algorithm::ES256 => decoding_key_from_ec_jwk(jwk)?,
            Algorithm::RS256 => decoding_key_from_rsa_jwk(jwk)?,
            _ => unreachable!(),
        };

        // Validate claims
        let mut validation = Validation::new(alg);
        validation.validate_exp = true;
        validation.leeway = 60;
        validation.set_issuer(&[&self.expected_iss]);
        validation.set_audience(&[&self.expected_aud]);

        let data = jsonwebtoken::decode::<SupabaseClaims>(token, &decoding_key, &validation)?;

        let claims = data.claims;
        let email = claims
            .email
            .ok_or_else(|| anyhow::anyhow!("email not found in token"))?;
        let role = claims.role.unwrap_or_else(|| "authenticated".to_string());

        Ok(UserInfo {
            id: claims.sub,
            email,
            role,
        })
    }
}

// Global verifier instance
pub static SUPABASE_VERIFIER: Lazy<RwLock<Option<Arc<SupabaseVerifier>>>> =
    Lazy::new(|| RwLock::new(None));

pub async fn init_supabase_verifier(project_ref: &str, expected_aud: &str) {
    let verifier = Arc::new(SupabaseVerifier::new(project_ref, expected_aud));
    let mut guard = SUPABASE_VERIFIER.write().await;
    *guard = Some(verifier);
}

pub async fn get_supabase_verifier() -> anyhow::Result<Arc<SupabaseVerifier>> {
    let guard = SUPABASE_VERIFIER.read().await;
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Supabase verifier not initialized"))
}
