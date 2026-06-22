use serde::{Deserialize, Serialize};

/// Long-term knot memory (`[memory]` in hi.toml).
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_enabled")]
    pub enabled: bool,
    #[serde(default = "default_owner_id")]
    pub owner_id: String,
    #[serde(default = "default_max_inject_chars")]
    pub max_inject_chars: usize,
    #[serde(default = "default_inject_clarity_threshold")]
    pub inject_clarity_threshold: f32,
    #[serde(default = "default_decay_enabled")]
    pub decay_enabled: bool,
    #[serde(default = "default_decay_half_life_days")]
    pub decay_half_life_days: f32,
    #[serde(default = "default_extract_after_turn")]
    pub extract_after_turn: bool,
    /// 仅在本回合命中记忆信号（如「记住/我叫/我喜欢」）或内容体量达到
    /// `extract_turn_min_tokens` 时才抽结；false 则退回旧的「每轮无条件抽取」。
    #[serde(default = "default_extract_after_turn_cue_only")]
    pub extract_after_turn_cue_only: bool,
    /// 本回合新内容达到该 token 估算量也触发抽结（0 = 关闭体量触发）。
    #[serde(default = "default_extract_turn_min_tokens")]
    pub extract_turn_min_tokens: usize,
    #[serde(default = "default_extract_on_compress")]
    pub extract_on_compress: bool,
    #[serde(default = "default_memory_search_enabled")]
    pub memory_search_enabled: bool,
    /// 暴露 `memory_write` 工具，允许 Agent 在回合内主动记录长期记忆。
    #[serde(default = "default_memory_write_tool")]
    pub memory_write_tool: bool,
    #[serde(default = "default_inject_baseline_only")]
    pub inject_baseline_only: bool,
    #[serde(default = "default_inject_baseline_max_chars")]
    pub inject_baseline_max_chars: usize,
    #[serde(default = "default_max_search_results")]
    pub max_search_results: usize,
    #[serde(default = "default_max_search_chars")]
    pub max_search_chars: usize,
}

fn default_decay_enabled() -> bool {
    true
}

fn default_decay_half_life_days() -> f32 {
    30.0
}

fn default_memory_enabled() -> bool {
    true
}

fn default_owner_id() -> String {
    "local".into()
}

fn default_max_inject_chars() -> usize {
    2000
}

fn default_inject_clarity_threshold() -> f32 {
    0.35
}

fn default_extract_after_turn() -> bool {
    true
}

fn default_extract_after_turn_cue_only() -> bool {
    true
}

fn default_extract_turn_min_tokens() -> usize {
    200
}

fn default_extract_on_compress() -> bool {
    true
}

fn default_memory_search_enabled() -> bool {
    true
}

fn default_memory_write_tool() -> bool {
    true
}

fn default_inject_baseline_only() -> bool {
    true
}

fn default_inject_baseline_max_chars() -> usize {
    800
}

fn default_max_search_results() -> usize {
    10
}

fn default_max_search_chars() -> usize {
    3000
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_memory_enabled(),
            owner_id: default_owner_id(),
            max_inject_chars: default_max_inject_chars(),
            inject_clarity_threshold: default_inject_clarity_threshold(),
            decay_enabled: default_decay_enabled(),
            decay_half_life_days: default_decay_half_life_days(),
            extract_after_turn: default_extract_after_turn(),
            extract_after_turn_cue_only: default_extract_after_turn_cue_only(),
            extract_turn_min_tokens: default_extract_turn_min_tokens(),
            extract_on_compress: default_extract_on_compress(),
            memory_search_enabled: default_memory_search_enabled(),
            memory_write_tool: default_memory_write_tool(),
            inject_baseline_only: default_inject_baseline_only(),
            inject_baseline_max_chars: default_inject_baseline_max_chars(),
            max_search_results: default_max_search_results(),
            max_search_chars: default_max_search_chars(),
        }
    }
}
