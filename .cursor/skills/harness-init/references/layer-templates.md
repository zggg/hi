# 架构分层模板

常见技术栈的分层参考模板。必须根据**真实**目录结构调整 —— 通过 import 模式发现，而不是强行套模板。

## Web Frontend（React / Vue / Svelte）

```
types/        → No app imports（纯定义）
utils/        → No app imports（纯函数）
lib/          → types/ only（clients、configs）
services/     → lib/, types/（业务逻辑）
hooks/states/ → lib/, services/, types/（状态管理）
components/   → hooks/, lib/, types/（UI）
pages/routes/ → components/, hooks/, lib/, types/（入口点）
```

## Backend API（Express / FastAPI / Rails）

```
types/models/ → No app imports（数据定义）
config/       → types/ only
db/repo/      → config/, types/（数据访问）
services/     → db/, config/, types/（业务逻辑）
middleware/   → services/, config/, types/（请求处理）
routes/       → services/, middleware/, types/（HTTP handlers）
```

## Java Backend（Spring Boot 传统分层）

适用于大多数经典 Spring Boot + Gradle 项目，尤其是按职责分包的单体服务。

```
controller/        → application/, service/, dto/（HTTP 入口）
application/       → service/, domain/, dto/（用例编排，可选层）
service/           → repository/, domain/, converter/, dto/（业务逻辑）
repository/        → po/, entity/, mapper/, config/（数据访问）
domain/            → No upward imports（领域模型与领域服务）
po/entity/         → No upward imports（持久化对象）
converter/mapper/  → domain/, po/, dto/（对象转换）
config/security/   → 作为外围支持层，被上层使用但不反向依赖业务层
```

常见约束：

- `controller` 不直接访问 `repository`
- `repository` 不依赖 `service`
- `domain` 不依赖 `controller/service/repository`
- `converter/mapper` 不承载业务编排

## Java Backend（Spring Boot DDD / 模块化分层）

适用于按领域或模块拆分的 Spring Boot 项目。**不要**把它当作所有老项目的默认迁移目标；只有在空仓库、用户显式要求、或现有仓库已经接近该结构时才采用。

```
interfaces/        → application/, dto/（controller, facade, api）
application/       → domain/, dto/（用例编排）
domain/            → No outward framework imports（实体、值对象、领域服务）
infrastructure/    → domain/, config/（repository impl, mq, external clients）
shared/            → util, common exceptions, shared types（仅基础共享）
```

常见约束：

- `interfaces` 不直接依赖 `infrastructure`
- `domain` 不依赖 Spring MVC、JPA、MyBatis 等框架实现
- `application` 负责编排，不直接承载持久化细节
- `infrastructure` 实现技术细节，但不反向驱动上层业务规则

## Java Backend（按业务域分包）

如果项目结构更像：

```
order/
user/
payment/
common/
```

这种按业务域组织的项目，不能直接套传统 `controller/service/repository/domain` 横向分层模板。

应先判断：

- 每个业务域内部是否还存在自己的 controller/service/repository 分层
- 是否需要按“域内分层 + 域间边界”建模
- 是否应该先生成说明文档和检查清单，而不是立刻生成统一的 ArchUnit 边界规则

## 选择规则（Java / Spring Boot）

按下面顺序判断：

1. 如果仓库为空，或用户明确要求标准多模块结构，可优先采用 `common/domain/application/infrastructure/interfaces/bootstrap`
2. 如果现有仓库已经明显接近该结构，可沿用并补齐
3. 如果现有仓库是单模块、传统分层或按业务域分包，优先按现状建模，不要强行迁移到标准多模块结构

## Full-Stack（Next.js / Nuxt / SvelteKit）

```
types/        → No app imports
lib/          → types/ only（共享工具）
db/           → lib/, types/（数据库）
services/     → db/, lib/, types/（业务逻辑）
components/   → lib/, types/（UI primitives）
features/     → components/, services/, lib/, types/（功能模块）
app/pages/    → features/, components/, lib/, types/（路由）
```

## Monorepo（Turborepo / Nx）

```
packages/types/   → No internal imports
packages/config/  → types/
packages/db/      → config/, types/
packages/api/     → db/, config/, types/
packages/ui/      → types/ only
packages/web/     → ui/, api/, types/
```

## OpenAI 原始模型

Harness engineering 文章中的经典模型：

```
Types → Config → Repo → Service → Runtime → UI
```

每一层只能导入左侧的层。

**Providers** 用来承载跨领域关注点（auth、connectors、telemetry、feature flags）。Provider 是注入这些共享能力的唯一机制；禁止跨领域直接导入。Provider 应包装外部服务或共享能力，并通过清晰接口暴露给其他层，而不破坏依赖方向。
