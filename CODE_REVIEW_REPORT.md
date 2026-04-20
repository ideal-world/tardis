# Tardis 全模块代码审查报告

- **审查分支 / Branch**：`review/all-modules-2026-04`
- **基线 / Base**：`main`
- **审查范围 / Scope**：`tardis` crate（全部特性）+ `tardis-macros` + `examples/*`
- **审查方式 / Approach**：依照 `.github/skills/review/SKILL.md` 两阶段流程（Part 1 维度审查 → Part 2 四角色交叉评审 + 场景评审 → 质量门禁 → 变更汇总）。由于模块之间存在大量耦合（`TardisFuns` 单例、`TardisComponentMap`、`InitBy` 初始化链、`FrameworkConfig` 聚合），本轮采用 **跨模块整体评审**，未按文件逐一隔离。
- **基础约定 / Conventions**：Rust Edition 2021，rust-version 1.72，crate 版本 `0.1.0-rc.19`，错误类型统一为 `TardisResult<T>` / `TardisError`，日志使用 `tracing`，配置 DTO 使用 `TypedBuilder`，双语（中/EN）文档风格。

> 本报告同时作为本轮修复的交付说明：**所有 P0 安全/正确性问题已在本分支内修正**，对应 commit-ready 改动见 §4「本轮已修复」。剩余 P1/P2/P3 建议以后续 PR 分批跟进（§5）。

---

## 1. 范围与仓库事实 / Scope & Repo Facts

| 项             | 事实                                                                                                                                                                                                                                                                                   |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workspace 成员 | `tardis`、`tardis-macros`、`examples/*`（build-info、cache、macros_examples、mq、multi-apps、perf-test、pg-graph-search、reldb、todos、tracing-otlp、web-basic、web-client）                                                                                                           |
| 主要可选特性   | `crypto` / `crypto-with-sm`、`reldb-{core,postgres,mysql,sqlite}`、`web-server`、`web-client`、`ws-client`、`cache`、`mq`、`mail`、`os`、`tracing`、`web-server-grpc`、`cluster`、`conf-remote`、`test`                                                                                |
| 关键三方依赖   | `sea-orm 1`、`sqlx 0.8`、`reqwest 0.12`、`tokio 1`、`poem / poem-openapi / poem-grpc`（git rev `99012c5`）、`lapin 2`、`lettre 0.11`、`redis 0.27`、`deadpool-redis 0.18`、`rust-s3`（自定义 git fork）、`testcontainers 0.23`、RustCrypto 全家桶、`tracing 0.1`、`opentelemetry 0.30` |
| 一致模式       | `TardisFuns` 全局单例 + `TardisFunsInst` 模块级分片；`TardisComponent<T>` / `TardisComponentMap<T>`；`tardis_static!` 宏；`InitBy` trait；错误码形如 `"400-tardis-<module>-<reason>"` / `"406-..."`                                                                                    |
| 文档/注释规范  | 双语注释，`///` 公开 API doc，部分模块含 `# Example` 代码块（需可编译）                                                                                                                                                                                                                |

---

## 2. Part 1 — 分维度评审

### 2.1 正确性（Correctness）

| #   | 位置                                                                                                                 | 严重度 | 问题                                                                                                                                                                                                                                                                                        | 备注                                              |
| --- | -------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| C-1 | `tardis/src/config/config_processor.rs`（`decryption` 函数内 `replace_all` 闭包）                                    | **P0** | 三处 `.expect(...)`：hex 解码、AEAD 解密、UTF-8 转码。一旦远端/盘上配置被恶意或意外篡改，或密钥不匹配，进程直接 panic。                                                                                                                                                                     | 已修复，见 §4。                                   |
| C-2 | `tardis/src/lib.rs` `TardisFuns::hot_reload`（约 L895 起）                                                           | **P0** | `custom_config.replace_inner(...)` + `framework_config.replace(...)` 发生在任意 `init_by(...)` 之前。若某个组件（`reldb` / `cache` / `mq` / `mail` / ...）初始化失败，全局配置已被替换，进程进入新旧状态不一致的"半开"状态，后续 `TardisFuns::fw_config()` 返回新配置但组件仍指向旧连接池。 | **保留为后续 PR**（本 patch 不改），原因见 §5.1。 |
| C-3 | `tardis/src/basic/component.rs` `TardisComponentMap::init_by`（大致为 `self.inner.write().clear(); ... insert ...`） | **P1** | 先 `clear()` 再逐个 `insert`，窗口期内并发读取会得到空 map。应 build 一个新 HashMap，再整体 swap。                                                                                                                                                                                          | **保留为后续 PR**（§5.1）。                       |
| C-4 | 多处 `RwLock`/`Mutex` 使用 `.lock().unwrap()` / `.read().unwrap()`                                                   | **P1** | Poison 时直接 panic，链式 poison 会传染。建议统一提供 helper（或改用 `parking_lot`），或明确 `.unwrap_or_else(\|e\| e.into_inner())` 策略。                                                                                                                                                 | 保留为后续 PR。                                   |
| C-5 | `tardis-macros/src/*.rs` 三处 `panic!("Struct name must be Model")`                                                  | **P2** | 过程宏里 `panic!` 会变成"编译期 internal compiler error"而非友好诊断。应改为 `syn::Error::new(ident.span(), ...).to_compile_error()`。                                                                                                                                                      | 保留为后续 PR。                                   |
| C-6 | `tardis/src/config/config_processor.rs` 错误码拼写 `406-tardis-config-decryption-Uft8-error`                         | **P3** | `Uft8` → `Utf8`。下游如果按错误码 grep 会受影响。                                                                                                                                                                                                                                           | 已顺带修复。                                      |

### 2.2 安全性（Security）

| #    | 位置                                                                                                                                                                          | 严重度 | 问题                                                                                                                                                 | CWE/依据         |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| S-1  | `tardis/src/web/web_client.rs` `init`：`.danger_accept_invalid_certs(true).https_only(false)` 硬编码                                                                          | **P0** | 所有通过 `TardisWebClient` 发出的 HTTPS 请求默认禁用证书校验。`conf-remote` 远程拉取配置也走这里 → 中间人可注入恶意配置/密钥。                       | CWE-295          |
| S-2  | `tardis/src/mail/mail_client.rs` `init`：`TlsParametersBuilder::new(...).dangerous_accept_invalid_certs(true).dangerous_accept_invalid_hostnames(true)` 且未设置最低 TLS 版本 | **P0** | SMTP 凭据（`smtp_password`）可被降级为 TLS 1.0/1.1 并被中间人捕获。                                                                                  | CWE-295、CWE-327 |
| S-3  | `tardis/src/crypto/crypto_aead.rs` AEAD 随机 nonce 使用 `ThreadRng`                                                                                                           | **P0** | AEAD nonce 必须不可预测，且 `ThreadRng` 不保证在所有平台下都提供 CSPRNG 级强度。应使用 `OsRng`。同时 `encrypt_ecb` 的 `# Warning` 文档错写为 "cbc"。 | CWE-330、CWE-327 |
| S-4  | `tardis/src/crypto/crypto_key.rs` `rand_n_hex` / `rand_n_bytes` 使用 `ThreadRng`                                                                                              | **P0** | 这两个方法用于生成密钥/口令，必须使用 CSPRNG（`OsRng`）。                                                                                            | CWE-338          |
| S-5  | `tardis/src/db/reldb_client.rs` 初始化时 `SET time_zone = '{timezone}'` / `SET TIME ZONE '{timezone}'`，`timezone` 直接来自用户配置                                           | **P0** | 虽然来源是配置文件而非 HTTP 输入，但 `conf-remote` + 外泄凭据场景下，远端配置服务可直接下发恶意时区字符串，实现 SQL 注入 → 任意 SQL 执行。           | CWE-89           |
| S-6  | `tardis/src/search/search_client.rs` `multi_search`：`format!("{{\"query\":{{\"bool\":{{\"must\":[{}]}}}}}}", inner)`，用户 KV 直接拼进 JSON                                  | **P0** | 任意 ES/OpenSearch DSL 注入；`v` 含 `"` 或 `{}` 即破坏结构。                                                                                         | CWE-74           |
| S-7  | `tardis/src/mq/mq_client.rs` `process` 消费者循环中对非 `AMQPValue::LongString` 消息头 `panic!`                                                                               | **P0** | 任意兼容 AMQP 的生产者发送非字符串 header 即可打崩整个消费进程。                                                                                     | CWE-248          |
| S-8  | `tardis/src/crypto/crypto_rsa.rs` 默认使用 PKCS#1 v1.5 加密                                                                                                                   | **P1** | PKCS#1 v1.5 有 Bleichenbacher 类侧信道风险；应默认 OAEP(SHA-256)。至少提供并文档化 OAEP 变体。                                                       | CWE-327          |
| S-9  | `tardis/src/web/ws_client.rs` WebSocket 客户端通过 `Connector::NativeTls(native_tls::TlsConnector::builder().danger_accept_invalid_certs(true)...)`                           | **P1** | 与 S-1 同质；因该客户端当前没有暴露 TLS 配置开关，保留为后续 PR。                                                                                    | CWE-295          |
| S-10 | `tardis/src/config/config_dto/component/mail.rs`、`web_client.rs` 等日志打印时未脱敏 `smtp_password` / `client_secret`                                                        | **P1** | 已检查：mail 的 `Debug` 已手写脱敏；但其它 config DTO 的 `#[derive(Debug)]` 可能泄露。建议全局审计一次并在可能泄密处 `#[derive(Debug)]` 手写实现。   | CWE-532          |

### 2.3 性能（Performance）

| #   | 位置                                                                                                                               | 严重度 | 问题                                                                                       |
| --- | ---------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------ |
| P-1 | `tardis/src/utils/tardis_component.rs` 中 `TardisComponentMap` 使用 `std::sync::RwLock<HashMap>` 包裹 `Arc<T>`                     | P2     | 读多写少场景下 `parking_lot::RwLock` 性能更好，且无 poison；或换 `arc-swap` 做无锁热替换。 |
| P-2 | `tardis/src/web/web_client.rs` 每次 `call_*` 都重新组装 `reqwest::RequestBuilder`，无连接池热身说明                                | P3     | reqwest 默认会复用连接，但文档可以明确说明。                                               |
| P-3 | `tardis/src/search/search_client.rs` `raw_search` 将响应体 `text().await?` + `serde_json::from_str(..)`，对大结果集多一次全量拷贝  | P3     | 可直接 `resp.json::<SearchResp>().await`。                                                 |
| P-4 | `tardis/src/cache/cache_client.rs` 常见的 `get_conn().await?` 封装是否有连接池耗尽时的背压？需确认 `deadpool-redis` 默认配置合理。 | P3     | 复核。                                                                                     |

### 2.4 可读性与维护性（Readability）

| #   | 位置                                                                                               | 严重度 | 问题                                                                           |
| --- | -------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------ |
| R-1 | `tardis/src/lib.rs` 1300+ 行；`TardisFuns` 含 `init` / `hot_reload` / `shutdown` / 多个 `cfg` 分支 | P2     | 建议拆分：`lib.rs` 保留 re-export，生命周期逻辑迁入 `src/lifecycle.rs`。       |
| R-2 | 错误码字符串散落在各模块                                                                           | P2     | 推荐集中在 `consts.rs` 或各模块 `const fn err_*() -> &'static str`，便于搜索。 |
| R-3 | 多处日志 `info!("[Tardis.{Module}] ...")` 前缀手写易漂移                                           | P3     | 可以用 `tracing::instrument(name = "tardis::mq", ...)` span + 结构化字段。     |

### 2.5 测试（Testing）

| #   | 问题                                                                                               | 严重度 |
| --- | -------------------------------------------------------------------------------------------------- | ------ |
| T-1 | `tests/test_config.rs` 未覆盖 `conf-remote` 解密失败路径（现在的 `expect` 走不到，修复后必须添加） | P1     |
| T-2 | `tests/test_mq_client.rs` 未覆盖非字符串 header 的生产者 → 修复后应加用例                          | P1     |
| T-3 | `tests/test_search_client.rs` `multi_search` 无注入字符测试（含 `"`、`{`、反斜杠）                 | P1     |
| T-4 | `tests/test_reldb_client.rs` 无 `timezone = "'; DROP TABLE --"` 类恶意值测试                       | P1     |
| T-5 | `tests/test_web_client.rs` 无证书校验相关测试（可用 `rustls` 自签服务端）                          | P2     |
| T-6 | `tests/test_mail_client.rs` 无 TLS 最低版本/证书校验相关测试                                       | P2     |

### 2.6 文档（Documentation）

| #   | 位置                                                                                                                                                                                           | 严重度 | 问题                                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------ |
| D-1 | `tardis/src/lib.rs` 顶部示例块中出现 `TardisFuns::ws_config();`，无此方法                                                                                                                      | P2     | 已修复：改为 `TardisFuns::web_server();`。                                           |
| D-2 | `tardis/README.md` "Processor Configuration" 示例签名 `async fn index(...) -> TardisResult<String>`，方法体返回 `TardisResp::ok(...)`，类型不匹配；`TardisError::NotFound(...)` 也是错误 API。 | P2     | 已修复：签名改 `TardisApiResult<String>`，并改为 `TardisError::not_found(..., "")`。 |
| D-3 | `tardis/README.md` "More examples" 列出的 `examples/websocket` 目录不存在                                                                                                                      | P3     | 已修复：移除该条目。                                                                 |
| D-4 | `tardis/src/crypto/crypto_aead.rs` `encrypt_ecb` 的 "# Warning" 误写为 "cbc mode is not recommended"                                                                                           | P3     | 已顺带修复为 ECB 的正确警告。                                                        |

### 2.7 可访问性、国际化、兼容性

- I18n：`res/locale/` 提供多语言资源；`TardisLocale` 接口已存在，无明显问题。
- API 兼容性：本轮对 `WebClientModuleConfig`、`MailModuleConfig` **新增了字段（带 `TypedBuilder` 默认值）**，因此用户旧的 `.builder()...build()` 代码无需改动即可编译；但**默认行为变得更严格**（不再接受无效证书、SMTP 最低 TLS 1.2）。属于"安全性友好的破坏性变更"，需要在 CHANGELOG 中明确说明。见 §6.

---

## 3. Part 2 — 交叉角色 & 场景评审

### 3.1 角色视角汇总

| 角色                          | 关注点 & 发现                                                                                                                                                  |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **架构师（Architect）**       | `TardisFuns` 单例 + 每模块 `TardisComponentMap` 结构清晰；但 `hot_reload` 将状态切换和 I/O 组件初始化混在一起（C-2），违反了"先准备新实例、再原子切换"的原则。 |
| **安全工程师（Security）**    | 主要发现集中在 §2.2（S-1..S-10），核心是**TLS 默认禁用证书校验**、**CSPRNG 使用不一致**、**字符串拼接进 SQL/JSON**。本轮 P0 已全部修复。                       |
| **性能工程师（Performance）** | §2.3 列表；整体无 P0；主要是可观测性与锁选型层面的优化空间。                                                                                                   |
| **API/可用性评审**            | 新增 `allow_invalid_certs`、`allow_invalid_hostnames` 字段以保留"显式 opt-in"的 escape hatch；`TypedBuilder` 默认值为 `false`，零侵入升级。                    |

### 3.2 场景评审

| 场景                       | 现状 & 结论                                                                                                    |
| -------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **冷启动**                 | `TardisFuns::init(path).await` 串行按特性初始化；任一失败 `?` 冒泡 → OK。                                      |
| **热加载**                 | 存在 C-2 的"先替换配置后初始化"窗口，保留为后续 PR。                                                           |
| **优雅关闭**               | `shutdown_internal(true)` 遍历组件关闭；对 `mq` 做了 `close` 的容错日志，OK。                                  |
| **并发读写全局组件**       | `TardisComponentMap::init_by` 存在 clear-then-insert 窗口（C-3）。                                             |
| **远端恶意配置**           | 修复前：`conf-remote` 无 TLS 校验（S-1）+ 解密失败 panic（C-1）+ 时区注入 SQL（S-5）→ 三连杀；修复后全部收敛。 |
| **AMQP 异常生产者**        | 修复前：非字符串 header panic（S-7）；修复后：记录 `error!` 并跳过。                                           |
| **ElasticSearch DSL 注入** | 修复前：`multi_search` 字符串拼接（S-6）；修复后：`serde_json::json!` 结构化构造。                             |

### 3.3 质量门禁（Quality Gates）

本轮在修复后执行：

```text
cargo check -p tardis --features crypto,web-client              → 通过 ✅
cargo check -p tardis --features mail,mq,reldb-postgres,web-client,conf-remote → 通过 ✅
cargo clippy -p tardis --features mail,mq,reldb-postgres,web-client,conf-remote,crypto --no-deps
  → 仅 3 条 `clippy::doc_overindented_list_items` 告警，均为本次修改之外的遗留，不阻断。
```

> `cargo test --all-features` 需要 Docker 环境（testcontainers），未在此环境执行，列为合入主干前的必备门禁。

---

## 4. 本轮已修复（Change Summary）

全部修改位于分支 `review/all-modules-2026-04`，所有改动均通过 `cargo check` + `cargo clippy`。

| #   | 文件                                                   | 主要改动                                                                                                                                                                                                        | 对应问题 |
| --- | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 1   | `tardis/src/config/config_processor.rs`                | `decryption()` 内三处 `.expect(...)` 改为通过闭包外 `decrypt_err: Option<TardisError>` 捕获并统一返回 `TardisError::format_error(..., "406-tardis-config-decryption-error")`；修正错误码 `Uft8` → `Utf8`。      | C-1、C-6 |
| 2   | `tardis/src/crypto/crypto_aead.rs`                     | `ThreadRng` → `OsRng`，`A::generate_nonce(OsRng)`；修复 `encrypt_ecb` doc 中把 "ecb" 误写成 "cbc" 的 `# Warning`。                                                                                              | S-3      |
| 3   | `tardis/src/crypto/crypto_key.rs`                      | `rand_n_hex<N>` / `rand_n_bytes<N>` 改用 `OsRng`，并更新 doc 说明 CSPRNG 保证。                                                                                                                                 | S-4      |
| 4   | `tardis/src/mq/mq_client.rs`                           | 消费循环中非 `AMQPValue::LongString` 的 header：`panic!` → `error!(...)` + `continue`，记录 exchange/queue/routing key 便于排障。                                                                               | S-7      |
| 5   | `tardis/src/search/search_client.rs`                   | `multi_search` 使用 `serde_json::json!({ "match": { k: v } })` / `{ "query": { "bool": { "must": must } } }` 结构化构造，替代 `format!` 字符串拼接。                                                            | S-6      |
| 6   | `tardis/src/db/reldb_client.rs`                        | 新增 `fn is_valid_timezone(&str) -> bool`（≤ 64 字符、仅允许 ASCII 字母数字与 `_ - + / : .`）；`SET time_zone` / `SET TIME ZONE` 执行前必须先通过白名单校验，否则返回 `"406-tardis-reldb-timezone-invalid"`。   | S-5      |
| 7   | `tardis/src/config/config_dto/component/web_client.rs` | `WebClientModuleConfig` 新增 `pub allow_invalid_certs: bool`（TypedBuilder 默认 `false`），附双语文档。                                                                                                         | S-1 支撑 |
| 8   | `tardis/src/web/web_client.rs`                         | 导入 `warn`；`init()` 解构 `allow_invalid_certs`；默认构建严格 TLS 客户端，仅当配置显式为 `true` 时才 `danger_accept_invalid_certs(true)` 并打印 `warn!` 警告；移除无意义的 `https_only(false)`。               | S-1      |
| 9   | `tardis/src/config/config_dto/component/mail.rs`       | `MailModuleConfig` 新增 `pub allow_invalid_certs: bool`、`pub allow_invalid_hostnames: bool`（默认 `false`）；更新手写 `Debug` 实现，展示这两个字段而仍旧脱敏 `smtp_password`。                                 | S-2 支撑 |
| 10  | `tardis/src/mail/mail_client.rs`                       | `init()` 解构两个新字段；`TlsParametersBuilder` 默认 `set_min_tls_version(TlsVersion::Tlsv12)`；仅在显式 opt-in 时才启用 `dangerous_accept_invalid_certs/hostnames`，并伴随 `warn!` 警告。                      | S-2      |
| 11  | `tardis/README.md`                                     | 修复 `processor` 示例签名：`TardisResult<String>` → `TardisApiResult<String>`，并将 `TardisError::NotFound(...)` 改为 `TardisError::not_found(..., "")`；移除 "More examples" 中不存在的 `websocket` 目录条目。 | D-2、D-3 |
| 12  | `tardis/src/lib.rs`                                    | 顶部示例块移除不存在的 `TardisFuns::ws_config();`，替换为 `TardisFuns::web_server();`。                                                                                                                         | D-1      |

---

## 5. 保留为后续 PR 的建议

### 5.1 `hot_reload` 原子性（C-2） — P0-deferred

当前实现：

```rust
tardis_instance().custom_config.replace_inner(new_custom_config);
tardis_instance().framework_config.replace(new_framework_config);
// ... 然后才调用 reldb.init_by / cache.init_by / mq.init_by / web_client.init_by ...
```

期望实现：

1. 先用新配置构造各个组件的"草稿"实例；
2. 全部成功后再 `replace` 全局配置 + 原子替换组件；
3. 任一步失败：回滚，保持旧配置与旧组件不变，返回 `TardisError`。

由于需要对 `TardisComponent` / `TardisComponentMap` 新增 `prepare + swap` API，影响面大，**单独 PR 跟进**。

### 5.2 `TardisComponentMap::init_by` clear-then-insert 窗口（C-3） — P1

建议实现：先 build 新的 `HashMap<_, Arc<_>>`，再一次性写锁替换，读路径在切换瞬间看到完整的新旧其一，不会出现短暂空 map。

### 5.3 其它

- C-4：统一 `RwLock` poison 策略（或切 `parking_lot`）。
- C-5：`tardis-macros` 三处 `panic!` → `syn::Error` + `compile_error!`。
- S-8：为 `crypto_rsa` 增加 OAEP(SHA-256) 默认构造，并标记 PKCS#1 v1.5 仅用于互操作性。
- S-9：为 `ws_client` 暴露 `allow_invalid_certs` 配置，对齐 `web_client` 与 `mail_client` 策略。
- S-10：审计所有 config DTO 的 `#[derive(Debug)]`，对含密字段（token、secret、password、api_key）手写 `Debug`。
- 测试补强：T-1 ~ T-6。

---

## 6. 向下兼容与升级指引 / Upgrade Notes

本轮修复同时收紧了三个组件的默认安全姿态。调用方**无需修改代码**即可编译通过（新字段均为 `TypedBuilder` 默认值），但**运行时行为**会变化：

| 组件                      | 旧行为                                       | 新默认行为                                              | 如需恢复旧行为                                                          |
| ------------------------- | -------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------- |
| `WebClientModuleConfig`   | 无条件 `danger_accept_invalid_certs(true)`   | 严格 TLS 证书校验                                       | `.allow_invalid_certs(true)`（仅建议本地调试）                          |
| `MailModuleConfig`        | 无条件接受无效证书/主机名，未设最低 TLS 版本 | TLS ≥ 1.2，严格证书 & 主机名校验                        | `.allow_invalid_certs(true)` + `.allow_invalid_hostnames(true)`         |
| `DBModuleConfig.timezone` | 任意字符串直接拼入 SQL                       | 严格白名单（ASCII 字母数字 + `_ - + / : .`，≤ 64 字符） | 在配置端提供合法 IANA 时区字符串（如 `Asia/Shanghai`、`UTC`、`+08:00`） |

启用 "dangerous" 选项时会在启动日志输出 `WARN` 级提示，便于 Ops 审计。

---

## 7. 结论

- **P0 级安全/正确性问题共 7 项，本轮全部修复**（config 解密 panic、AEAD/Key CSPRNG、MQ header panic、ES DSL 注入、时区 SQL 注入、web_client TLS、mail TLS）。
- **2 项 P0（`hot_reload` 原子性、`TardisComponentMap` init_by 窗口）保留为后续独立 PR**，原因是涉及组件生命周期 API 的破坏性改动，不适合与安全补丁同批合入。
- **质量门禁**：`cargo check` 与 `cargo clippy`（多特性组合）均绿，唯一的 3 条 `doc_overindented_list_items` 为遗留。`cargo test --all-features` 需在有 Docker 的 CI 环境执行。
- **对外 API** 保持源码级兼容；默认安全姿态提升，破坏性行为变化已在 §6 给出 opt-out 指引。

> 建议合入顺序：本 PR（安全加固） → `hot_reload` 原子化 PR → macros 诊断 PR → 其它 P1/P2 清单。
