mod decay;
mod extract;
mod inject;
mod merge;
mod owner;
mod search;
mod types;

pub use decay::{decay_clarity, effective_clarity};
pub use extract::{extract_knots, format_excerpt, turn_has_memory_cue, ExtractOutcome, ExtractedKnot};
pub use inject::{build_injection, InjectResult, BASELINE_SEARCH_HINT};
pub use merge::{merge_extracted, MergeOutcome};
pub use owner::{resolve_owner, OwnerId};
pub use search::{
    extract_keywords, format_search_results, run_memory_search, search_knots,
    select_baseline_knots,
};
pub use types::{
    content_hash, Knot, KnotConfidence, KnotKind, KnotStatus, KnotVisibility, NewKnot,
    TaskStatus,
};