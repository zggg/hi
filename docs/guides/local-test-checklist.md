# hi 本地功能测试清单

> **完全没装过？** 先看 [install.md](install.md)，装好并能 `hi chat 你好` 再回来。  
> 已安装则从 **§1 引导配置大模型** 开始打勾。  
> **注意**：下文部分 UI 文案可能与当前 cliclack 向导略有差异；配置路径以 **`~/.hi/hi.toml`** 为准（非 `config.toml`），API Key 写入 hi.toml（非 `HI_API_KEY` 环境变量）。

---

## 0. 安装（首次必做）

见 **[docs/guides/install.md](install.md)**，核心三步：

```bash
source "$HOME/.cargo/env"
cd /path/to/hi
cargo install --path app
hi --help
```


| #   | 检查项                                   | 通过  |
| --- | ------------------------------------- | --- |
| 0.1 | `cargo --version` 有输出                 | [ ] |
| 0.2 | `hi --help` 有 tui/chat/gateway/config | [ ] |


未装 Rust 或下载慢 → install.md 第 1、3 步（含代理）。

---

## 1. 引导配置大模型

### 1.1 进入配置向导

```bash
cp ~/.hi/hi.toml ~/.hi/hi.toml.bak 2>/dev/null || true
hi setup
```


| #   | 检查项                                 | 通过  |
| --- | ----------------------------------- | --- |
| 1.1 | 出现「hi · 基础配置」或类似向导标题 | [ ] |


向导第一问 **工作目录** 建议不要用 `.`，单独建目录给工具测试用：

```bash
export HI_TEST_DIR="$HOME/hi-test-workspace"
mkdir -p "$HI_TEST_DIR"
echo "hello from hi" > "$HI_TEST_DIR/original.txt"
```

在向导里填：

```text
远程 workspace（预留）[$HI_TEST_DIR 的绝对路径，gateway 用；tui/chat 用当前目录]:
→ 例如 /Users/你/hi-test-workspace
```


| #   | 检查项                      | 通过  |
| --- | ------------------------ | --- |
| 1.2 | 工作目录已创建且含 `original.txt` | [ ] |


### 1.3 选择 Provider（五选一）

向导会显示：

```text
请选择大模型 Provider:
  DeepSeek — 官方 API（platform.deepseek.com）
  OpenAI 兼容接口 — OpenAI、Moonshot、自部署 OpenAI 兼容服务
  OpenAI Codex — 复用本地 Codex CLI 登录（ChatGPT），无需 API Key
  Anthropic Claude — Anthropic Messages API
  Ollama 本地推理 — 本机部署，无需 API Key
```

---

#### 方案 A：DeepSeek（推荐首次测试）

| 向导步骤 | 建议输入 |
| -------- | -------- |
| Provider | 选 **DeepSeek** |
| 模型 | 选 `deepseek-v4-flash`（回车）或 `deepseek-v4-pro` |
| API Key | 在 [platform.deepseek.com/api_keys](https://platform.deepseek.com/api_keys) 申请后粘贴 |
| 启用上下文自动压缩? | `Y` |
| （向导不再问企微） | 需要时另跑 `hi gateway setup` |

base_url 自动设为 `https://api.deepseek.com`，无需手动填写。

---

#### 方案 B：OpenAI 兼容 / 自部署 API

| 向导步骤 | 建议输入 |
| -------- | -------- |
| Provider | 选 **OpenAI 兼容接口** |
| API base_url | 你的地址（如 `http://127.0.0.1:8000/v1`） |
| model | 手动输入你的模型名 |
| API Key | 按服务商要求填写 |

---

#### 方案 B：Anthropic Claude


| 向导步骤          | 建议输入                |
| ------------- | ------------------- |
| 序号            | `2`                 |
| API base_url  | 留空（官方默认）            |
| API Key 环境变量名 | `ANTHROPIC_API_KEY` |
| 压缩 / 企微       | 同方案 A               |


```bash
export ANTHROPIC_API_KEY=sk-ant-...
```


| #    | 检查项                          | 通过  |
| ---- | ---------------------------- | --- |
| 1.3B | 已 export `ANTHROPIC_API_KEY` | [ ] |


---

#### 方案 C：本地 Ollama

```bash
ollama serve          # 另开终端，若未运行
ollama pull llama3.2
```


| 向导步骤          | 建议输入                         |
| ------------- | ---------------------------- |
| 序号            | `3`                          |
| Ollama 地址     | `http://localhost:11434`（回车） |
| API Key 环境变量名 | （跳过，ollama 不需要）              |



| #    | 检查项                       | 通过  |
| ---- | ------------------------- | --- |
| 1.3C | `ollama list` 能看到所选 model | [ ] |


---

### 1.4 确认配置文件

向导结束应看到：

```text
已写入 /Users/你/.hi/hi.toml
下一步:
  hi chat 你好
  hi
```

```bash
hi config
```


| #   | 检查项                                                      | 通过  |
| --- | -------------------------------------------------------- | --- |
| 1.4 | `~/.hi/hi.toml` 存在                                   | [ ] |
| 1.5 | `hi config` 中 `ai.provider` / `ai.model` / `ai.base_url` 正确 | [ ] |
| 1.6 | `workspace` 指向 `$HI_TEST_DIR`（gateway）；CLI 在 `$HI_TEST_DIR` 下执行 | [ ] |
| 1.7 | 打印了 `sessions.db` 路径                                     | [ ] |


**自部署改 model 示例**（若向导默认 model 不对）：

```bash
# 编辑 ~/.hi/hi.toml
# [ai]
# model = "你的模型名"
```

---

### 1.5 大模型连通性冒烟（必做）

```bash
hi chat 回复 OK 两个字母
```


| #   | 检查项                                 | 通过  |
| --- | ----------------------------------- | --- |
| 1.8 | 有正常 assistant 回复（非 error）           | [ ] |
| 1.9 | 故意 unset Key 再跑，报错含 missing env（可选） | [ ] |


**连通失败排查：**


| 现象               | 处理                              |
| ---------------- | ------------------------------- |
| missing API key  | `export` 的变量名与 `api_key_env` 一致 |
| 401 / 403        | Key 无效或 base_url 不对             |
| connection error | 检查代理、自部署服务是否启动                  |
| model not found  | 改 `hi.toml` 里 `ai.model`    |


---

## 2. `hi chat` — 对话与工具

### 2.1 四工具

```bash
hi chat 读取 original.txt 的内容
hi chat 创建 note.txt，内容为 test-write
hi chat 把 original.txt 里的 hello 改成 hi
hi chat 执行 ls -la 列出工作目录
```


| #   | 检查项                         | 通过  |
| --- | --------------------------- | --- |
| 2.1 | **read** 读到 `hello from hi` | [ ] |
| 2.2 | **write** 生成 `note.txt`     | [ ] |
| 2.3 | **edit** 修改 `original.txt`  | [ ] |
| 2.4 | **bash** 有 `[tool] bash ok` | [ ] |


### 2.2 REPL 多轮

```bash
hi chat
# you> 记住我叫小明
# you> 我叫什么？
# you> /quit
```


| #   | 检查项        | 通过  |
| --- | ---------- | --- |
| 2.5 | 同会话内记住「小明」 | [ ] |
| 2.6 | `/quit` 退出 | [ ] |


### 2.3 危险命令审批

```bash
hi chat 执行 rm -rf /tmp/hi-fake-test
# 出现审批提示后输入 n
```


| #   | 检查项                      | 通过  |
| --- | ------------------------ | --- |
| 2.7 | 触发 `[approval required]` | [ ] |
| 2.8 | 拒绝后不执行                   | [ ] |


---

## 3. 会话持久化（M3）

```bash
rm -f ~/.hi/data/sessions.db    # 可选：干净起点

hi chat 持久化口令 alpha-123
hi chat 我刚才的口令是什么？
```


| #   | 检查项                         | 通过  |
| --- | --------------------------- | --- |
| 3.1 | 第二次能答出 `alpha-123`          | [ ] |
| 3.2 | `~/.hi/data/sessions.db` 存在 | [ ] |


---

## 4. 会话隔离

```bash
hi chat chat 秘密 chat-secret-999
hi tui
# 问：chat 通道的秘密是什么？  → 应不知道
# Ctrl+C 退出
hi chat chat 秘密是什么？   → 应仍知道
```


| #   | 检查项             | 通过  |
| --- | --------------- | --- |
| 4.1 | TUI 不知道 chat 秘密 | [ ] |
| 4.2 | 再开 chat 仍记得     | [ ] |


---

## 5. 上下文压缩（M5）

编辑 `~/.hi/hi.toml`：

```toml
[context]
enabled = true
window_k = 8
compress_at_k = 4
protect_tail_k = 2
```

多轮 `hi chat` 直到出现 `[context compressed]`，测完改回 `window_k = 128`。


| #   | 检查项                   | 通过  |
| --- | --------------------- | --- |
| 5.1 | 出现 context compressed | [ ] |
| 5.2 | 压缩后仍可对话               | [ ] |


---

## 6. `hi tui`

```bash
hi tui
```


| #   | 操作         | 预期           | 通过  |
| --- | ---------- | ------------ | --- |
| 6.1 | 打开界面       | 显示 model、cwd | [ ] |
| 6.2 | 「列出当前目录文件」 | 有 tool 输出    | [ ] |
| 6.3 | 触发危险 bash  | Y/N 审批       | [ ] |
| 6.4 | Ctrl+C     | 正常退出         | [ ] |


---

## 7. 消息渠道 Gateway（已配 wecom 时）

可在 `hi gateway setup` 里配企微，或编辑 `~/.hi/hi.toml` 后：

```bash
export WECOM_SECRET=...
export RUST_LOG=info,hi_gateway=debug

hi gateway --check
hi gateway
```


| #   | 检查项              | 通过  |
| --- | ---------------- | --- |
| 7.1 | `--check` 订阅成功   | [ ] |
| 7.2 | 企微收到欢迎语 + 文本回复   | [ ] |
| 7.3 | 与 `hi chat` 会话不串 | [ ] |


详见 [wecom-gateway-integration.md](wecom-gateway-integration.md)。

---

## 8. 切换其他 Provider（可选）

重新跑向导或手改 `[ai]` 后，各测一条：

```bash
hi setup          # 或手改 hi.toml
export <对应_KEY>=...
hi chat ping
```


| #   | Provider            | 通过  |
| --- | ------------------- | --- |
| 8.1 | openai-compat / 自部署 | [ ] |
| 8.2 | anthropic + 工具调用    | [ ] |
| 8.3 | ollama              | [ ] |


---

## 附录 A. 环境准备（首次 clone 时）

```bash
source "$HOME/.cargo/env"
# 如需代理
export https_proxy=http://127.0.0.1:PORT http_proxy=http://127.0.0.1:PORT all_proxy=socks5://127.0.0.1:PORT

cargo build -p hi
hi --help
```

---

## 附录 B. 自动化测试（无需 LLM）

```bash
cargo test --workspace
cargo test -p architecture-tests
./scripts/check-consistency.sh
```


| #   | 检查项     | 通过  |
| --- | ------- | --- |
| B.1 | 全部 PASS | [ ] |


---

## 附录 C. 并发 / 边界（可选）


| #   | 场景                      | 通过  |
| --- | ----------------------- | --- |
| C.1 | gateway + chat 同时运行     | [ ] |
| C.2 | 错误 workspace / 当前目录不可访问时报错 | [ ] |
| C.3 | read 工作目录外路径被拒绝         | [ ] |


---

## 验收汇总


| 模块          | 从哪节开始  | 必测    | 通过  |
| ----------- | ------ | ----- | --- |
| **引导配置大模型** | **§1** | **是** | [ ] |
| chat + 四工具  | §2     | 是     | [ ] |
| SQLite 持久化  | §3     | 是     | [ ] |
| 会话隔离        | §4     | 是     | [ ] |
| 上下文压缩       | §5     | 建议    | [ ] |
| TUI         | §6     | 是     | [ ] |
| 消息渠道 Gateway | §7  | 有凭证   | [ ] |
| 多 Provider  | §8     | 按需    | [ ] |


---

## 最快路径（只验证大模型）

```bash
hi setup
hi config
hi chat 回复 OK
```

测试完把 `[ ]` 改成 `[x]` 留档即可。