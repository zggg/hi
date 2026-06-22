use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Context window management and lossy summarization (`[context]`).
///
/// All budget fields use **K tokens** (1 K ≈ 1_000 tokens): `128` means ~128K context.
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct ContextConfig {
    pub enabled: bool,
    /// Model context window (K tokens).
    pub window_k: u32,
    /// Start compression when estimated input exceeds this many K tokens.
    pub compress_at_k: u32,
    /// Keep the most recent ~N K tokens verbatim (walk backward from the end).
    pub protect_tail_k: u32,
    /// Reserve for this turn's model output and tool calls (K tokens).
    pub reserve_k: u32,
    /// Max chars returned by read/bash before truncation at the tool layer.
    pub tool_output_max_chars: usize,
    /// When emergency-trimming history, never shrink a message below this many chars.
    pub trim_keep_chars: usize,
    /// Max ReAct loops per user turn (LLM ↔ tool call cycles).
    pub max_tool_iterations: usize,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ContextConfigRaw {
    enabled: bool,
    window_k: Option<u32>,
    compress_at_k: Option<u32>,
    protect_tail_k: Option<u32>,
    reserve_k: Option<u32>,
    max_tokens: Option<usize>,
    compress_threshold: Option<f32>,
    keep_recent_turns: Option<usize>,
    tool_output_max_chars: Option<usize>,
    trim_keep_chars: Option<usize>,
    max_tool_iterations: Option<usize>,
}

impl Default for ContextConfigRaw {
    fn default() -> Self {
        Self {
            enabled: default_context_enabled(),
            window_k: None,
            compress_at_k: None,
            protect_tail_k: None,
            reserve_k: None,
            max_tokens: None,
            compress_threshold: None,
            keep_recent_turns: None,
            tool_output_max_chars: None,
            trim_keep_chars: None,
            max_tool_iterations: None,
        }
    }
}

fn default_context_enabled() -> bool {
    true
}

pub fn default_window_k() -> u32 {
    128
}

pub fn default_compress_at_k() -> u32 {
    96
}

pub fn default_protect_tail_k() -> u32 {
    32
}

pub fn default_reserve_k() -> u32 {
    16
}

fn default_tool_output_max_chars() -> usize {
    16_384
}

fn default_trim_keep_chars() -> usize {
    2_048
}

pub fn default_max_tool_iterations() -> usize {
    12
}

impl ContextConfig {
    pub fn window_tokens(&self) -> usize {
        self.window_k.saturating_mul(1000) as usize
    }

    pub fn compression_threshold_tokens(&self) -> usize {
        self.compress_at_k.saturating_mul(1000) as usize
    }

    pub fn protect_tail_tokens(&self) -> usize {
        self.protect_tail_k.saturating_mul(1000) as usize
    }

    pub fn reserve_tokens(&self) -> usize {
        self.reserve_k.saturating_mul(1000) as usize
    }

    fn from_raw(raw: ContextConfigRaw) -> Self {
        let mut window_k = raw.window_k.unwrap_or_else(default_window_k);
        let mut compress_at_k = raw.compress_at_k.unwrap_or_else(default_compress_at_k);
        let mut protect_tail_k = raw.protect_tail_k.unwrap_or_else(default_protect_tail_k);
        let reserve_k = raw.reserve_k.unwrap_or_else(default_reserve_k);

        if raw.window_k.is_none() {
            if let Some(max_tokens) = raw.max_tokens {
                window_k = tokens_to_k(max_tokens);
            }
        }
        if raw.compress_at_k.is_none() {
            if let Some(threshold) = raw.compress_threshold {
                compress_at_k = ((window_k as f32 * threshold) as u32).max(8);
            } else if let Some(max_tokens) = raw.max_tokens {
                compress_at_k = tokens_to_k(max_tokens * 8 / 10);
            }
        }
        if raw.protect_tail_k.is_none() {
            if let Some(turns) = raw.keep_recent_turns {
                // Legacy: ~4K tokens per user turn → K
                protect_tail_k = (turns as u32 * 4).max(8);
            }
        }

        Self {
            enabled: raw.enabled,
            window_k,
            compress_at_k,
            protect_tail_k,
            reserve_k,
            tool_output_max_chars: raw
                .tool_output_max_chars
                .unwrap_or_else(default_tool_output_max_chars),
            trim_keep_chars: raw.trim_keep_chars.unwrap_or_else(default_trim_keep_chars),
            max_tool_iterations: raw
                .max_tool_iterations
                .unwrap_or_else(default_max_tool_iterations),
        }
    }
}

fn tokens_to_k(tokens: usize) -> u32 {
    tokens.div_ceil(1000).max(8) as u32
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: default_context_enabled(),
            window_k: default_window_k(),
            compress_at_k: default_compress_at_k(),
            protect_tail_k: default_protect_tail_k(),
            reserve_k: default_reserve_k(),
            tool_output_max_chars: default_tool_output_max_chars(),
            trim_keep_chars: default_trim_keep_chars(),
            max_tool_iterations: default_max_tool_iterations(),
        }
    }
}

impl Serialize for ContextConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Out {
            enabled: bool,
            window_k: u32,
            compress_at_k: u32,
            protect_tail_k: u32,
            reserve_k: u32,
            max_tool_iterations: usize,
            #[serde(skip_serializing_if = "is_default_tool_output_max")]
            tool_output_max_chars: usize,
            #[serde(skip_serializing_if = "is_default_trim_keep")]
            trim_keep_chars: usize,
        }
        Out {
            enabled: self.enabled,
            window_k: self.window_k,
            compress_at_k: self.compress_at_k,
            protect_tail_k: self.protect_tail_k,
            reserve_k: self.reserve_k,
            max_tool_iterations: self.max_tool_iterations,
            tool_output_max_chars: self.tool_output_max_chars,
            trim_keep_chars: self.trim_keep_chars,
        }
        .serialize(serializer)
    }
}

fn is_default_tool_output_max(v: &usize) -> bool {
    *v == default_tool_output_max_chars()
}

fn is_default_trim_keep(v: &usize) -> bool {
    *v == default_trim_keep_chars()
}

impl<'de> Deserialize<'de> for ContextConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ContextConfigRaw::deserialize(deserializer).map(ContextConfig::from_raw)
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/context.rs"]
mod tests;
