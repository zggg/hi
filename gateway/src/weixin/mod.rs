//! 个人微信 iLink — HTTP 长轮询。

mod adapter;
pub mod gateway;
mod ilink;
pub mod login;
mod state;

pub use adapter::WeixinAdapter;
pub use ilink::QrCodeResponse;
pub use ilink::QrStatusKind;
pub use login::{fetch_qr_code, poll_qr_status, wait_for_qr_login, QrLoginResult};
