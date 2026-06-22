mod bash;
mod diff_preview;
mod edit;
mod memory_search;
mod memory_write;
mod output;
mod path_util;
pub use path_util::{FileAccess, ResolvedPath};
mod read;
mod registry;
mod tool;
mod write;

pub use output::limit_tool_output;

pub use bash::BashTool;
pub use edit::EditTool;
pub use memory_search::MemorySearchTool;
pub use memory_write::MemoryWriteTool;
pub use read::ReadTool;
pub use registry::ToolRegistry;
pub use tool::{MemoryToolDeps, Tool, ToolContext, ToolDefinition};
pub use write::WriteTool;

pub const TOOL_READ: &str = "read";
pub const TOOL_WRITE: &str = "write";
pub const TOOL_EDIT: &str = "edit";
pub const TOOL_BASH: &str = "bash";
pub const TOOL_MEMORY_SEARCH: &str = "memory_search";
pub const TOOL_MEMORY_WRITE: &str = "memory_write";

pub fn default_tool_names() -> [&'static str; 5] {
    [
        TOOL_READ,
        TOOL_WRITE,
        TOOL_EDIT,
        TOOL_BASH,
        TOOL_MEMORY_SEARCH,
    ]
}
