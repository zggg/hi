pub const TOOL_READ: &str = "read";
pub const TOOL_WRITE: &str = "write";
pub const TOOL_EDIT: &str = "edit";
pub const TOOL_BASH: &str = "bash";

pub fn default_tool_names() -> [&'static str; 4] {
    [TOOL_READ, TOOL_WRITE, TOOL_EDIT, TOOL_BASH]
}
