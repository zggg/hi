# hi 命令清单（待审核）

> 状态：**草案** — 供审核是否收敛、合并、删除。  
> 最后更新：2026-06-22（配置格式 `[channels.*]`；TUI 内联视口；补充 `/model` / `memory_write`）

---

## 1. 对话内斜杠命令（chat / tui 输入框）

### 1.1 共享命令（`core/src/session.rs` → `parse_session_command`）

| 命令 | hi chat | hi tui | 作用 |
|------|:-------:|:------:|------|
| `/reset` | ✓ | ✓ | 清空 Agent 可见上下文（transcript 仍保留在 DB，`in_context=0`） |
| `/clear` | ✓ | ✓ | 同 `/reset`（别名） |
| `/compact` | ✓ | ✓ | 强制上下文裁剪 + LLM 摘要 |
| `/quit` | ✓ | ✗ | 退出 chat REPL |
| `/exit` | ✓ | ✗ | 同 `/quit`（别名） |

### 1.2 TUI 专属（`tui/src/slash.rs`）

| 命令 | hi chat | hi tui | 作用 |
|------|:-------:|:------:|------|
| `/model` | ✗（报错提示用 TUI） | ✓ | 切换 `[ai.providers]` 激活实例 |

**边界说明**

| 场景 | 是否支持斜杠 |
|------|--------------|
| `hi chat` REPL | ✓（无 `/model`） |
| `hi tui` | ✓（无 `/quit`，用 `Ctrl+C` 退出） |
| `hi chat …` 单轮 | ✗（`/reset` 等会当作普通用户消息交给 LLM） |
| `hi gateway` / 远程渠道 | ✗（用户只能发普通文本） |

**当前数量**：共享 5 个字符串（3 个有效 + 2 别名）；TUI 额外 `/model`。

---

## 2. TUI 快捷键（非斜杠，但属于「会话内操作」）

TUI 采用**内联视口 + 终端原生 scrollback**（完成的对话写入 scrollback，鼠标/系统滚动条可用）。状态栏文案见 `tui/src/app.rs` → `status_line()`。

**空闲时（默认）：**

```text
←→ 移动光标 · Enter 发送 · Shift+Enter 换行 · Ctrl+C 退出 · {model}
```

**斜杠菜单 / 模型子菜单打开时：**

| 状态 | 提示 |
|------|------|
| `/model` 子菜单 | `↑↓ 选择模型 · Enter 激活 · Esc 取消` |
| 其它斜杠菜单 | `↑↓ 选择 · Tab/Enter 填入输入框` |

| 按键 | 作用 |
|------|------|
| `Enter` | 发送输入（含斜杠命令） |
| `Shift+Enter` | 换行（多行输入） |
| `←` / `→` | 移动输入框光标 |
| `↑` / `↓` | 斜杠/模型菜单中选择；输入框多行时上下移动 |
| `Home` / `End` | 光标到行首/行尾 |
| `Backspace` / `Delete` | 删除字符 |
| `Ctrl+C` | 运行中：中断当前回合；空闲：退出 TUI |
| `Esc` | 清空输入框；关闭斜杠/模型菜单；审批弹窗时 = 拒绝 |
| `Tab` | 斜杠菜单中填入选中项 |
| `y` / `Y` | bash/文件审批通过 |
| `n` / `N` | bash/文件审批拒绝 |

审批弹窗打开时，按键优先交给审批处理；此时 `Ctrl+C` 不作为中断/退出处理。

---

## 3. Shell CLI（`hi` 子命令）

### 3.1 顶层入口

| 命令 | 说明 |
|------|------|
| `hi` | 默认等同 `hi tui` |
| `hi tui` | 本地终端 UI |
| `hi chat` | 命令行 REPL（无尾部参数） |
| `hi chat 词1 词2 …` | 单轮对话后退出 |
| `hi chat --session ID` | 指定 chat 会话 ID（默认 `chat:main`） |
| `hi chat --session ID 词1 词2 …` | 指定会话的单轮对话 |
| `hi setup` | 交互配置 LLM、workspace（`[context]` 用默认值写入） |
| `hi gateway` | 消息渠道网关 |
| `hi gateway --check` | 连接预检，成功后退出 |
| `hi config` | 查看当前配置（密钥脱敏） |
| `hi session` | 会话 transcript / 压缩历史 |
| `hi memory` | 结绳长期记忆 |

### 3.2 `hi gateway` 子命令

| 子命令 | 说明 |
|--------|------|
| `setup` | 交互配置消息渠道（企业微信 / 飞书 / 个人微信 iLink） |
| `start` | 后台启动（release 构建下 `hi gateway` 默认行为） |
| `stop` | 停止后台 gateway |
| `restart` | 重启 |
| `status` | 进程状态 |
| `reload` | 热加载 `hi.toml` 的 `[ai]` + `[tools.approvals]`（仅 Unix，SIGUSR1；macOS 可用） |
| `run` | 前台运行（debug 构建下 `hi gateway` 默认行为） |

### 3.3 `hi session` 子命令

| 子命令 | 说明 |
|--------|------|
| `list` | 所有会话及消息计数 |
| `show [--session ID] [--context]` | 查看 transcript；`--context` 仅 Agent 可见行 |
| `export [--session ID] [-o file|--output file]` | 导出 JSON |
| `compressions [--session ID]` | 压缩事件列表 |
| `compression-show <id>` | 单次压缩详情 |
| `purge --session ID --confirm` | **永久删除** 会话及全部消息 |

默认 session id 示例：

| 入口 | session_id |
|------|------------|
| `hi tui` | `tui:main` |
| `hi chat` | `chat:main`（可用 `--session ID` 覆盖） |
| 默认企微用户 A | `wecom:A` |
| 多账号企微用户 A | `wecom:{account}:A` |

### 3.4 `hi memory` 子命令

| 子命令 | 说明 |
|--------|------|
| `list [--all] [--owner ID]` | 列出活跃结绳 |
| `show <id>` | 结绳详情 |
| `add "文本" [--kind KIND] [--confirmed] [--permanent] [--owner ID]` | 手动添加 |
| `forget <id>` | 软删除 |
| `reinforce <id> [--permanent]` | 强化（clarity → 1.0） |
| `extract [--session ID] [--owner ID]` | LLM 从 transcript 抽取结绳 |

---

## 4. Agent 工具（非用户命令）

用户无需记忆；由 Agent 在回合内自动调用。

| 工具 | 说明 |
|------|------|
| `read` | 读工作目录内文件 |
| `write` | 写文件 |
| `edit` | 编辑文件 |
| `bash` | 执行 shell（危险命令需审批） |
| `memory_search` | 按需检索长期记忆（`[memory]` 启用且 `memory_search_enabled = true`） |
| `memory_write` | Agent 主动写入长期记忆（`memory_write_tool = true`） |

---

## 5. 数量汇总

| 类别 | 条目数 | 日常对话典型用量 |
|------|--------|------------------|
| 对话斜杠 | 6（共享 3 有效 + 2 别名 + TUI `/model`） | 0～1 |
| TUI 快捷键 | ~12 | 2～3 |
| CLI 子命令 | ~25+（含 flag） | 安装/排错时偶尔 |
| Agent 工具 | 6（memory 工具按配置启用） | 0（Agent 自用） |

---

## 6. 待审核：收敛方向（草案）

以下为可选原则，**尚未实施**，审核时可勾选或改写。

### 6.1 对话斜杠

- [ ] **只保留 `/reset`**（或 `/clear` 二选一），去掉 `/compact`（改由 Agent 自动裁剪）
- [ ] chat 与 tui **统一退出方式**（都支持 `/quit` 或都只支持 `Ctrl+C`）
- [ ] 单轮 `hi chat …` 是否也要识别 `/reset`（目前不识别）

### 6.2 CLI 与斜杠分工

- [ ] 运维操作（清空上下文、看 transcript、purge）**只留 `hi session`**，对话里不再加斜杠
- [ ] `hi session reset-context` 替代对话内 `/reset`（若砍掉斜杠）

### 6.3 文档与发现性

- [ ] `hi --help` 不展开全部子命令，只链到本文档
- [ ] chat/tui 启动时不打印斜杠列表，或只提示 `Ctrl+C` / 帮助链接

### 6.4 审核记录（你来填）

| 日期 | 决定 | 备注 |
|------|------|------|
| | | |

---

## 7. 相关文档

- [install.md](install.md) — 从零安装
- [setup.md](setup.md) — 开发环境
- [local-test-checklist.md](local-test-checklist.md) — 功能自测
