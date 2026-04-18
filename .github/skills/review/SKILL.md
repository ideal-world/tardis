---
name: review
description: "Use when: reviewing code in the ideal-world/tardis repository, including `tardis`, `tardis-macros`, feature-gated modules, example crates, tests, public API changes, async/runtime safety, security, performance, documentation, or PR quality. 适用于 `ideal-world/tardis` 仓库的完整代码审查：范围界定 → 仓库约定 → 基础审查 → 交叉评审 → 质量门禁 → 变更汇总。"
---

# review — Tardis 仓库代码审查规范

> 面向 AI 助手的 `ideal-world/tardis` 仓库审查指南。

---

## §1 审查准则

1. **先界定范围，再进入细节**：优先确认审查对象是 `tardis`、`tardis-macros`、某个 `examples/*` crate、某组 tests，还是一次跨 crate 变更；不要默认把整个 workspace 都拉进来。
2. **先确认仓库事实，再下结论**：先看 `Cargo.toml`、`lib.rs` / `main.rs`、README、feature、测试入口、example 用法与导出链，避免按别的项目经验脑补结构。
3. **以 Tardis 语境为准**：本仓库是 Rust workspace，核心约定通常围绕 `TardisFuns`、`TardisResult`、`TardisError`、`tardis::log` / `tracing`、feature gate、README 驱动文档、example crate 与集成测试展开。
4. **默认审查优先，修复要有依据**：如果用户要求“review”而非“直接改”，优先输出问题和建议；只有在用户明确要求修复，或问题非常明确且改动小、风险低时，才同步修改代码 / 测试 / 文档。
5. **最小变更、成套更新**：除非证据明确，不主动改业务语义和仓库风格；一旦修复问题，优先同步更新测试、README、crate 文档、feature 说明或示例。
6. **两阶段完整执行**：必须依次执行 **Part 1（基础审查）** 与 **Part 2（交叉评审与模拟）**，最后输出变更汇总；禁止只做一半就收工。

### 审查总览

```
Part 1 — 基础审查
  逐维度（§2-§11）审查代码 / feature / 测试 / 文档 → 记录问题并视情况修复 → 输出审查清单
      ↓ 自动进入
Part 2 — 交叉评审与模拟
  4 角色独立评审 → 交叉质疑 → 场景模拟 → 修复遗漏问题 → 质量门禁
      ↓
变更汇总
  合并 Part 1 + Part 2 结论，按优先级输出已修复 / 建议保留 / 合规确认
```

---

> **审查维度（§2 – §11）** — 以下各节为本仓库审查检查项参考，由执行流程中的 Part 1 / Part 2 驱动使用。

---

## §2 架构与结构审查

- [ ] 先确认变更所在层级：workspace、crate、module、feature、example、test、README，不把“库代码审查”误当成“业务 service 审查”
- [ ] `tardis` 以库为中心：`lib.rs` 负责公共导出、feature 边界、crate 级文档与入口组织，不承载难以维护的实现细节
- [ ] `tardis-macros` 与 `tardis` 的职责边界清晰：宏 crate 负责 derive / proc-macro 能力，主 crate 负责运行时与公共 API
- [ ] `examples/*` 作为使用样例与 smoke reference：示例应体现真实使用方式，而不是复制一套内部实现
- [ ] 公共导出最小化：`pub`、re-export、feature 暴露面克制，不把内部实现意外变成稳定 API
- [ ] feature 边界清晰：`#[cfg(feature = ...)]`、optional dependency、`required-features` 测试保持一致，不制造默认构建与特性构建行为漂移
- [ ] 目录与模块命名贴合仓库现实，例如 `basic` / `cache` / `config` / `crypto` / `db` / `mail` / `mq` / `os` / `search` / `web`，不要套用别处的 `api/serv/domain` 模板

---

## §3 命名与可读性审查

- [ ] **命名三问**通过：看名字知职责？不会与 feature / module / public API 混淆？6 个月后仍能快速理解？
- [ ] crate、module、feature、类型、测试名语义一致，尤其是 `Tardis*` 前缀的公共类型与 capability 名称是否统一
- [ ] 避免含糊命名（如 `data` / `info` / `handler` / `process` / `tmp`），特别是在公共 API、config DTO、宏参数和测试夹具里
- [ ] 长函数可读：重要分支拆分为私有函数或辅助方法，避免在 `init` / `hot_reload` / client 初始化路径中堆叠过深逻辑
- [ ] 返回路径清晰，不在 `Ok(...)` / `Err(...)` / `Some(...)` / `None` 分支中塞过度复杂逻辑
- [ ] `use` 组织稳定，外部依赖、本 crate 模块、trait 引入分组合理；公共 re-export 要能解释“为什么需要暴露”

---

## §4 类型与错误处理审查

- [ ] 对外 API 优先复用仓库既有模式，如 `TardisResult<T>`、`TardisError`、已有 dto / error 封装，而不是另起一套错误风格
- [ ] 避免在库代码主路径滥用 `unwrap` / `expect` / `panic!`；测试、示例、显式证明安全的静态初始化路径除外，但要能说明理由
- [ ] 错误向上透传时保留上下文，不吞错、不把底层失败静默转换成成功
- [ ] `?`、`map_err`、`From` / `Into` 转换使用合理，错误语义不被扭曲
- [ ] feature 关闭时不暴露无效 API；feature 打开时类型与导出关系能自洽
- [ ] 并发原语、共享状态与 `'static` 生命周期使用清晰，不靠“刚好编过”掩盖语义问题

---

## §5 注释与文档注释审查

- [ ] 公共导出 API、关键 trait、入口函数、宏行为有必要文档
- [ ] crate 级文档、README、示例、doc comment 与真实能力一致；`lib.rs` 引入 README 时尤其要注意重复和冲突
- [ ] 关键类型 / trait / 初始化入口至少有一句话说明职责和使用边界
- [ ] 示例只在真正帮助使用者时补充，避免编造伪示例或过期示例
- [ ] 保持仓库既有文档风格一致：本仓库常见中英双语说明，审查时不要机械要求“必须全中文”或“必须全英文”
- [ ] 注释优先解释“为什么 / 前置条件 / feature 约束”，而不是逐行复述代码

---

## §6 性能审查

### 时间复杂度

- [ ] **串行 await**：循环中的异步调用是否必须串行；能批量化或延迟初始化时避免无意义串行等待
- [ ] **重复计算**：热路径中是否重复解析配置、构造 client、序列化 JSON、拼接字符串、执行正则或 feature 相关初始化
- [ ] **不必要复制**：是否存在明显多余的 `clone()`、`to_string()`、`to_owned()`、大对象搬运

### 空间复杂度

- [ ] **全局状态增长**：静态 `HashMap`、组件缓存、模块实例池是否可能无限增长，是否有生命周期或替换策略
- [ ] **大对象处理**：查询、配置快照、批量结果、日志字符串是否一次性载入或复制过多数据

### I/O 与运行时

- [ ] **阻塞调用**：异步上下文中是否执行阻塞 I/O、长时间 CPU 计算或 `std::thread::sleep`
- [ ] **连接复用**：DB / HTTP / MQ / WebSocket / cache / mail / os client 是否复用而不是每次新建
- [ ] **资源释放**：连接、订阅、后台任务、web server、MQ client、reload 旧资源能否在关闭或异常路径释放

### 代码结构

- [ ] **热路径对象创建**：`TardisFunsInst`、client、provider、config wrapper 是否被重复创建而不是复用
- [ ] **锁使用正确**：避免持锁 `.await`、大粒度锁包裹慢操作、无界等待
- [ ] **复杂度可维护**：超长函数、超深嵌套、重复 feature 分支是否需要抽象或拆分

---

## §7 并发、全局状态与分布式语义审查

### 全局状态与初始化

- [ ] `init` / `init_conf` / `hot_reload` / `shutdown` 等入口在重复调用、并发调用、异常回滚场景下语义清晰
- [ ] `lazy_static!`、静态实例、组件容器、信号量等共享状态是否存在竞态风险（check-then-act、重复注册、重复关闭）
- [ ] 对“单例 + 热更新 + 多模块实例”组合场景，状态切换是否可预期

### 多节点 / 分布式语义（按需检查）

- [ ] 仅在涉及 `cache` / `mq` / `ws-client` / `web-server` / `cluster` / shared state 时检查多节点一致性；不要把所有模块都按分布式系统处理
- [ ] 不把必须跨节点一致的业务状态错误地放在进程内缓存里
- [ ] 网络失败后的重试、重连、重新订阅不会制造重复消息、重复初始化或资源泄露

### 任务与资源生命周期

- [ ] 后台任务、监听器、订阅、连接在 shutdown、reload、drop、测试结束时具备退出条件
- [ ] 异常路径能释放旧 client / server / subscription，不留下幽灵任务

---

## §8 安全审查

### 输入与注入

- [ ] **SQL / 查询注入**：原生 SQL、查询构造、搜索 DSL、对象存储路径、外部 URL 处理是否避免字符串拼接输入
- [ ] **命令 / 路径风险**：脚本调用、文件访问、对象存储 key、本地路径是否允许注入或路径逃逸
- [ ] **反序列化 / 配置污染**：环境变量、配置文件、HTTP 参数、MQ 负载、搜索请求是否做边界校验

### 凭据与敏感信息

- [ ] 无硬编码密钥、token、密码、真实连接串
- [ ] 敏感字段不会直接写进日志、错误消息、示例配置、测试夹具或 README
- [ ] 配置热加载、调试日志、trace 字段输出前已考虑脱敏

### 特性相关安全

- [ ] 涉及 `crypto` 时，算法、随机数、密钥处理不引入明显弱实现或不安全默认值
- [ ] 涉及 `web-server` / `web-client` / `os` 时，校验 SSRF、上传下载边界、外部回调地址、对象访问范围
- [ ] 涉及 `unsafe`、proc-macro、外部 git 依赖或 FFI 时，额外检查边界与供应链风险

---

## §9 日志与可观测性审查

- [ ] 优先使用 `tardis::log` / `tracing` 体系，而不是散落的 `println!` / `eprintln!`（示例或特殊边界除外）
- [ ] 日志级别符合语义：生命周期与初始化用 info/error，细节诊断用 debug/warn，避免刷屏式 info
- [ ] 关键路径具备足够上下文（模块、feature、对象标识、阶段），但不泄露敏感信息
- [ ] 错误日志可定位：至少能看出发生位置、动作与失败原因
- [ ] 文档与日志风格保持一致即可；不要为了审查而强推与文件现有风格冲突的语言规范

---

## §10 测试审查

- [ ] 测试入口符合 Rust 习惯：单元测试放模块内，集成测试放 `tardis/tests/`，宏或编译期行为优先走相应测试入口
- [ ] feature 相关能力有对应测试或 `required-features` 约束，避免“代码看起来对，但测试根本跑不到”
- [ ] 正常路径、边界路径、错误路径均有覆盖；必要时断言错误类别、关键消息或状态变化
- [ ] 触及公共 API、宏行为、README 示例、example crate 时，审查是否需要同步补测试或示例验证
- [ ] 涉及 DB / cache / MQ / OS / search 时，测试资源初始化与清理明确；尽量复用现有 testcontainers 模式
- [ ] 避免脆弱断言：随机性、时序依赖、外部网络、日志全文匹配、平台特定路径等都要谨慎

---

## §11 文档与仓库配套审查

### README / crate 文档

- [ ] 根 README 与 crate README 至少说明用途、能力、feature、运行 / 测试方式与限制条件
- [ ] `tardis/src/lib.rs` 通过 README 生成 crate 文档时，README 内容不能与实际 API、feature、examples 脱节
- [ ] 公共 API、feature、配置项、初始化方式变化后，README / 示例 / 测试应同步更新

### 仓库配套文件

- [ ] `Cargo.toml`、feature 描述、example 配置、workflow、脚本与实现保持一致
- [ ] 新增 feature、环境变量、依赖、必需工具（如 `protoc`）时有对应说明
- [ ] 如果变更跨多个 crate，需检查调用方 README、example crate 和 docs.rs 入口是否需要同步

---

> **执行流程** — 以下为本 skill 的执行过程，依次完成 Part 1 → Part 2 → 变更汇总。

---

## Part 1 — 基础审查

> 按 §2-§11 逐维度审查代码，发现问题后记录风险，并根据用户意图决定是仅输出建议，还是同步进行小而可验证的修复。

### 执行步骤

1. **界定范围**：说明审查对象、crate 边界、feature、example、测试入口、运行方式与风险面。
2. **确认仓库上下文**：查看相关 `Cargo.toml`、README、`lib.rs` / `main.rs`、feature、tests、examples、宏入口或导出链。
3. **逐维度审查**：按 §2-§11 检查源码、测试、文档、配置与 feature 边界，必要时确认调用链。
4. **处理发现**：
   - 若用户要求 review-only：记录问题等级、位置、证据、建议。
   - 若用户要求修复或问题足够明确且低风险：做小改动，并同步更新测试 / 文档。
5. **输出清单**：每个维度输出 checkbox 结果，标记 ✅ 合规 / ❌ 不合规（已修复） / ⚠️ 需注意 / ⏸ 暂未处理。

### 维度对照

| 维度 | 章节 | 审查要点 |
|------|------|---------|
| 架构与结构 | §2 | workspace / crate 边界、导出面、feature 组织、example 角色 |
| 命名与可读性 | §3 | 命名三问、一致性、函数拆分、re-export 与 use 组织 |
| 类型与错误处理 | §4 | `TardisResult` / `TardisError` 模式、避免 panic、feature 类型边界 |
| 注释与文档 | §5 | README / doc comment / 双语风格 / 示例有效性 |
| 性能 | §6 | 串行 await、clone 开销、client 复用、资源释放 |
| 并发与全局状态 | §7 | init / reload / shutdown 语义、静态状态、任务生命周期 |
| 安全 | §8 | 注入、凭据、配置污染、特性相关安全 |
| 日志与可观测性 | §9 | `tardis::log`、日志级别、上下文、脱敏 |
| 测试 | §10 | feature 覆盖、集成测试、example / macro / README 校验 |
| 文档与配套 | §11 | README、Cargo feature、workflow、示例与实现一致性 |

### Part 1 完成标志

- 所有 §2-§11 维度均已审查
- P0/P1 问题已修复，或明确标记为阻塞 / review-only 输出
- 已输出各维度 checkbox 清单

> Part 1 完成后，**自动进入 Part 2**。

---

## Part 2 — 交叉评审与模拟

> 用 4 个专家角色重新审视代码，交叉质疑结论，并以 Tardis 仓库常见场景做模拟，补齐 Part 1 遗漏的问题。

### 阶段 1：4 角色独立评审

| 角色 | 关注领域 |
|------|---------|
| **架构师** | workspace / crate 结构、导出边界、feature 组织、公共 API 稳定性（§2） |
| **运行时与性能专家** | async、锁、资源释放、全局状态、连接复用、reload / shutdown 语义（§6-§7） |
| **安全专家** | 配置 / 输入边界、凭据泄露、URL / SQL / 路径风险、crypto 与依赖边界（§8） |
| **API / 文档 / 测试审查专家** | 命名、类型、doc、README、examples、feature tests、可维护性（§3-§5, §9-§11） |

每个角色给出 **高价值发现**：

- 优先列出 P0/P1；如果没有，则明确写“未发现明显高风险问题”
- 尽量指向具体文件、函数、feature 或测试入口
- 不强行凑问题数量，禁止把风格偏好包装成高优先级缺陷

### 阶段 2：交叉质疑

至少进行 1 轮交叉质疑：

- 对争议风险补充证据
- 区分“真实缺陷”“仓库约定偏离”“未来优化建议”
- 如果某条结论优先级判断过重或过轻，主动修正

### 阶段 3：场景模拟

分析代码在以下场景的行为：

| 场景 | 分析要点 |
|------|---------|
| **高并发初始化** | 多任务同时 `init` / `hot_reload` / `shutdown` — 是否竞态、重复注册、重复关闭？ |
| **feature 组合变化** | 默认 feature、最小 feature、目标 feature 打开 / 关闭后，导出、测试、示例是否一致？ |
| **依赖不可用** | DB / Redis / MQ / SMTP / S3 / HTTP / search 不可用时，错误是否准确、资源能否回收？ |
| **配置错误** | 环境变量覆盖、缺字段、值越界、格式错误、热更新失败时，行为是否清晰？ |
| **恶意或异常输入** | 超长字符串、特殊字符、非法路径、脏 JSON、异常 URL、不可反序列化 payload 是否被正确处理？ |
| **资源耗尽** | 连接池打满、任务堆积、日志爆量、缓存膨胀、长生命周期静态对象是否可能失控？ |
| **文档消费者视角** | 用户只看 docs.rs / README / examples 时，是否会被过期示例、缺 feature 说明或错误导出误导？ |

### 阶段 4：问题汇总

合并 4 角色评审 + 场景模拟发现的问题，与 Part 1 已处理项去重，按 P0/P1/P2/P3 排序。

### 阶段 5：修复遗漏

针对 Part 2 新发现的 P0/P1 问题：

1. 给出修复方案
2. 实施代码 / 测试 / 文档的成套修改（若用户要求 review-only，则至少明确列出建议与原因）
3. 能补测试的必须补测试；若无法自动化覆盖，说明原因与替代验证方式

### 阶段 6：二次审查

确认 P0/P1 已清零，或在 review-only 模式下已被清楚标记为未修改风险项，且未引入新问题。

### 阶段 7：质量门禁

优先使用与改动范围匹配的 Rust 校验链，避免无差别全仓库轰炸：

1. `cargo fmt --all -- --check`，或按目标 package 校验格式
2. 针对目标 crate / feature 做 `cargo clippy`（feature 变更时优先校验受影响组合）
3. 针对目标 crate / integration tests / macro tests / examples 做 `cargo test`

必要时补充：

- `cargo check`
- 对 public API / feature gate / README 示例变更执行更贴近使用方的校验
- 对 `tardis-macros` 变更关注编译期测试或展开相关验证
- 仅文档改动时，至少保证内容与仓库事实一致

P0 或 P1 未清零 → 回到阶段 5。

### 阶段 8：文档完善

- 参考 §11，同步更新 README、`Cargo.toml` 的 feature / 依赖说明、example 文档、测试说明与 docs.rs 入口内容。

---

## 变更汇总与输出格式

> Part 1 + Part 2 全部完成后，输出本次审查的合并结论。

### 问题优先级

| 等级 | 含义 | 处理 |
|------|------|------|
| **P0** | 数据损坏 / 安全漏洞 / 服务崩溃 / 死锁 / 关键资源泄漏 / 明确错误公共 API | 必须立即修复或明确阻塞 |
| **P1** | 竞态 / 错误吞没 / feature 边界失效 / 明显行为不一致 / 关键测试缺失 | 本轮修复或明确列为高优先级风险 |
| **P2** | 性能瓶颈 / 可维护性问题 / 文档与示例不一致 / 复用性不足 | 建议修复 |
| **P3** | 风格 / 命名 / 注释 / 轻量结构优化 | 顺手修复 |

### 单项问题格式

```
### {等级} 问题标题
- 位置：`src/...` / `tests/...` / `examples/...` / `Cargo.toml` / `README.md`
- 问题：具体描述
- 风险：后果 / 影响
- 建议或修复：实际执行的改动；如果未修复，说明原因与后续建议
```

### 变更汇总清单

最终输出至少包含：

1. **已修复项**：列出 Part 1 + Part 2 中实际修改的问题（若本轮是 review-only，可写“未修改”）
2. **保留建议项**：P2/P3 或 review-only 场景下未修复但建议后续处理的项
3. **合规确认项**：审查通过、无需改动的维度汇总

格式示例：

```
## 变更汇总

### 已修复（N 项）
| # | 等级 | 文件 | 改动摘要 |
|---|------|------|---------|
| 1 | P1 | `tardis/src/lib.rs` | 修复 feature 导出边界 |
| 2 | P2 | `tardis/README.md` | 补充 feature 使用说明 |

### 保留建议（M 项）
| # | 等级 | 位置 | 建议 |
|---|------|------|------|
| 1 | P2 | `examples/web-basic/src/main.rs` | 建议补充错误路径示例 |

### 合规确认
- ✅ 架构与结构（§2）
- ✅ 安全（§8）
- ✅ 测试（§10）
```

---

## 示例触发语句

- "审查 `tardis` crate 的公共 API 设计"
- "review `tardis-macros` 的 derive 宏实现质量"
- "检查 `web-server` feature 的导出与测试覆盖"
- "审查 `TardisFuns::hot_reload` 的并发安全性"
- "review `examples/web-basic` 是否符合当前 README 用法"
- "做一次完整的 Tardis 仓库代码审查"
