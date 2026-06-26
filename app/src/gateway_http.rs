use hi_core::{ChannelsConfig, Result};

/// Generate and persist `[channels.http].token` when enabled and empty.
///
/// Author: gz
pub fn ensure_http_token() -> Result<()> {
    let mut channels = ChannelsConfig::load()?;
    let mut http = channels.http_account_config("default")?;
    if !http.enabled {
        return Ok(());
    }
    if !http.token.trim().is_empty() {
        return Ok(());
    }
    http.token = generate_token();
    channels.set_http_account("default", http.clone());
    channels.save()?;
    tracing::info!(
        token = %http.token,
        "HTTP gateway token generated and saved to hi.toml"
    );
    Ok(())
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_token_is_64_hex_chars() {
        let token = generate_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
