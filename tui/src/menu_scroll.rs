/// 菜单滚动窗口起始下标，保证 `selected` 在可见区内。
pub fn menu_window_start(len: usize, selected: usize, max_rows: usize) -> usize {
    if len <= max_rows {
        return 0;
    }
    selected
        .saturating_sub(max_rows.saturating_sub(1))
        .min(len.saturating_sub(max_rows))
}

#[cfg(test)]
#[path = "../test/unit/menu_scroll.rs"]
mod tests;
