//! Message-channel gateway adapters. Current: WeCom, Feishu, Weixin iLink.

pub mod adapter;
pub mod common;
pub mod feishu;
pub mod http;
pub mod run;
pub mod wecom;
pub mod weixin;

pub use adapter::ChannelAdapter;
pub use http::{reload_http_auth, shared_http_auth, SharedHttpAuth};
pub use run::{run_gateway, GatewayRunOptions};
