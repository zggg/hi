use std::time::Duration;

use hi_core::error::{Error, Result};

use crate::weixin::ilink::{IlinkClient, QrCodeResponse, QrStatusResponse};

pub use crate::weixin::ilink::QrStatusKind;

/// QR login result after user scans in WeChat.
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct QrLoginResult {
    pub bot_token: String,
    pub ilink_bot_id: String,
    pub ilink_user_id: String,
    pub base_url: String,
}

/// Author: gz
pub async fn fetch_qr_code(base_url: &str, bot_type: u32) -> Result<QrCodeResponse> {
    IlinkClient::new(base_url, "").fetch_qr_code(bot_type).await
}

/// Author: gz
pub async fn poll_qr_status(base_url: &str, qrcode: &str) -> Result<QrStatusResponse> {
    IlinkClient::new(base_url, "")
        .poll_qr_status(qrcode)
        .await
}

/// Poll until confirmed, expired, or timeout.
///
/// Author: gz
pub async fn wait_for_qr_login(
    base_url: &str,
    qrcode: &str,
    timeout: Duration,
    on_status: impl Fn(QrStatusKind),
) -> Result<QrLoginResult> {
    let deadline = std::time::Instant::now() + timeout;
    let client = IlinkClient::new(base_url, "");

    while std::time::Instant::now() < deadline {
        let status = client.poll_qr_status(qrcode).await?;
        on_status(status.kind());

        match status.kind() {
            QrStatusKind::Confirmed => {
                let bot_token = status
                    .bot_token
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::Message("weixin login: missing bot_token".into()))?;
                let ilink_bot_id = status
                    .ilink_bot_id
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::Message("weixin login: missing ilink_bot_id".into()))?;
                let ilink_user_id = status
                    .ilink_user_id
                    .unwrap_or_default();
                let resolved_base = status
                    .baseurl
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| base_url.to_string());
                return Ok(QrLoginResult {
                    bot_token,
                    ilink_bot_id,
                    ilink_user_id,
                    base_url: resolved_base,
                });
            }
            QrStatusKind::Expired => {
                return Err(Error::Message("weixin 二维码已过期，请重新生成".into()));
            }
            QrStatusKind::Wait | QrStatusKind::Scanned => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Err(Error::Message("weixin 扫码登录超时，请重试".into()))
}
