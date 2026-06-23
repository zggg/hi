//! Message-channel gateway adapters. Current: WeCom, Feishu, Weixin iLink.

pub mod adapter;
pub mod common;
pub mod feishu;
pub mod run;
pub mod wecom;
pub mod weixin;

pub use adapter::ChannelAdapter;
pub use run::run_gateway;
