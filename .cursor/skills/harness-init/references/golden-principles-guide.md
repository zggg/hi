# Golden Principles 指南

在 `docs/golden-principles/` 中放 3-5 个文档，每个 30-60 行，包含 DO 和 DON'T 示例。

## 如何编写

1. **先读真实代码模式。** 先发现，不要猜。
2. 每个文件只覆盖一个主题。
3. 先写规则，再给 DO/DON'T 代码示例。
4. 控制在 30-60 行，超出就拆文件。

## 常见候选主题

按技术栈挑选，不必全部都写：

| File | Topic | 何时使用 |
|------|-------|------|
| `IMPORTS.md` | 路径别名、排序、禁止深层相对路径导入 | Always |
| `NAMING.md` | 文件命名、导出约定 | Always |
| `ERROR_HANDLING.md` | 错误处理与报告 | Always |
| `TESTING.md` | 测什么、怎么测 | If tests exist |
| `DATA_FETCHING.md` | 数据获取与缓存 | Frontend |
| `LOGGING.md` | 日志约定 | If custom logger |

## Spring Boot + Gradle 项目推荐主题

如果目标项目主要是 Spring Boot + Gradle，优先考虑这些主题：

| File | Topic | 何时使用 |
|------|-------|------|
| `SPRING_LAYERS.md` | controller / application / service / repository / domain 的职责边界 | Always |
| `TRANSACTION.md` | 事务边界放在哪里，谁允许加 `@Transactional` | If persistence exists |
| `DTO_MAPPING.md` | DTO / entity / domain / po 之间如何转换 | If multiple model types exist |
| `EXCEPTION_HANDLING.md` | 业务异常、接口异常、全局异常处理约定 | Always |
| `JPA_USAGE.md` | 实体使用约束、懒加载、查询边界 | If JPA/Hibernate exists |
| `MYBATIS_USAGE.md` | mapper、xml、po 的职责边界 | If MyBatis exists |
| `API_LAYER.md` | controller 返回值、参数校验、接口风格 | If REST API exists |

### Spring Boot 项目默认优先组合

如果是典型的 Spring Boot 业务项目，优先生成这 4 份：

1. `SPRING_LAYERS.md`
2. `TRANSACTION.md`
3. `EXCEPTION_HANDLING.md`
4. `DTO_MAPPING.md`

这样可以先把最容易漂的几件事固定住：

- controller / service / repository 的职责边界
- 事务放在哪一层
- 异常如何向上抛和统一处理
- DTO / entity / domain / po 如何转换

### Spring Boot 主题写法建议

#### `SPRING_LAYERS.md`

重点写：

- Controller 只做参数接收、返回值组装、权限/校验入口
- Service / Application 负责业务编排
- Repository / Mapper 只做数据访问
- Domain 不反向依赖 Web 层或持久化实现

#### `TRANSACTION.md`

重点写：

- `@Transactional` 默认放在哪一层
- 哪些方法允许开启事务
- Controller / Repository 是否禁止直接声明事务
- 只读事务和写事务怎么区分

#### `EXCEPTION_HANDLING.md`

重点写：

- 业务异常、系统异常、参数异常怎么区分
- 哪些异常允许直接抛出
- 哪些异常必须转成统一响应
- 全局异常处理器负责什么，不负责什么

#### `DTO_MAPPING.md`

重点写：

- DTO / Entity / Domain / PO 各自职责
- 哪一层做对象转换
- 禁止在哪一层直接混用持久化对象和接口对象
- MapStruct / 手写 converter / mapper 的使用边界

## 模板

~~~markdown
# {Topic}

## Rule
{One-sentence rule statement}

## DO

```{lang}
// Good: {why this is correct}
{code example}
```

## DON'T

```{lang}
// Bad: {why this is wrong}
{code example}
```

## Exceptions
{When the rule doesn't apply, if any}
~~~
