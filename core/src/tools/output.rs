/// Cap tool stdout/file content before it enters agent context.
pub fn limit_tool_output(content: String, max_chars: usize) -> String {
    let total = content.chars().count();
    if total <= max_chars {
        return content;
    }
    let kept: String = content.chars().take(max_chars).collect();
    let omitted = total - max_chars;
    format!(
        "{kept}\n\n[output truncated: {omitted} chars omitted; read smaller ranges or narrow the command]"
    )
}

#[cfg(test)]
#[path = "../../test/unit/tools/output.rs"]
mod tests;
