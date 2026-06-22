# SECURITY.md 模板

用于生成 `docs/SECURITY.md`。根据 Phase 0 的真实发现填写，描述模式，不要写具体秘密。

## 排除规则

docs/SECURITY.md 绝不能包含：
- 真实密钥、API key 或 token
- 具体的密钥环境变量名（使用泛化描述）
- 内部基础设施细节（IP、内网域名、端口）
- 已知未修复漏洞的细节
- 存放凭证的精确文件路径

应使用泛化描述，例如：“数据库凭证从环境变量加载”，而不是 “DB_PASSWORD in .env”。

## 模板

<!-- AGENT INSTRUCTION: The exclusion rules above apply to ALL sections below.
     Never include specific environment variable names, credential file paths,
     internal hostnames/IPs, or unpatched vulnerability details anywhere. -->

~~~markdown
# Security

## Authentication

| Flow | Method | Where |
|------|--------|-------|
| {flow name} | {JWT / session / OAuth / API key} | {general location: middleware, gateway, etc.} |

## Authorization

{Describe the permission model: RBAC, ABAC, resource-based, etc.}
{Which layer enforces it — reference docs/architecture/LAYERS.md}

## Secrets Management

- **Storage:** {environment variables / secrets manager / vault — generic, no names}
- **Rotation:** {policy: manual / automated / on-deploy}
- **Access:** {who/what can read secrets — app runtime, developers, operators}

## Threat Model

| Threat | Mitigation | Status |
|--------|-----------|--------|
| {e.g., injection} | {e.g., parameterized queries} | {in place / planned} |
| {e.g., XSS} | {e.g., CSP headers + output encoding} | {in place / planned} |

## Dependencies

- Security-critical dependencies: {auth library, crypto library — names only, no versions with known CVEs}
- Dependency update policy: {Dependabot / manual / scheduled}

## Incident Response

- How to report: {general process}
- Escalation: {team / channel — no PII}
~~~

## 何时可以省略章节

- **没有认证：** 可跳过 Authentication 和 Authorization（例如 CLI 工具、静态站）
- **没有密钥：** 可跳过 Secrets Management（例如纯前端库）
- **内部工具：** Threat Model 可以更简短，重点写数据敏感度
