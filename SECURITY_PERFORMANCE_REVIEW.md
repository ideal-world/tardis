# Tardis 安全 & 性能专项审查报告

- **审查分支 / Branch**：`review/all-modules-2026-04`（承接上一轮全量审查）
- **基线 / Base**：`main`
- **审查范围**：`tardis` crate（全部 feature）+ `tardis-macros`
- **审查重点**：**安全（§8）+ 性能（§6）**（§2–§5、§7、§9–§11 已在 [CODE_REVIEW_REPORT.md](CODE_REVIEW_REPORT.md) 覆盖，本轮仅在有新发现时补充）
- **已核验**：所有问题均由 `grep` / 源码阅读复核通过，去除了首轮探查中两处不成立的判定（见 §4 "已核验剔除"）

> 说明：上一轮已修复的 7 项 P0 + 5 项 P1（TLS、CSPRNG、SQL/DSL 注入、MQ panic、config 解密 panic、README 漂移）不在本报告重复。本轮聚焦**新发现**的安全与性能风险。

---

## 1. 安全发现（Security）

### 1.1 结论速览

| #    | 标题                                                                                   | 等级   | 位置                                                                                                                                                              |
| ---- | -------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S-01 | `ConfCenterConfig` 存在重名遮蔽，`password` 未脱敏直接进入 debug 日志                  | **P0** | [tardis/src/config/config_dto.rs](tardis/src/config/config_dto.rs#L189-L224) × [tardis/src/config/config_processor.rs](tardis/src/config/config_processor.rs#L63) |
| S-02 | `ws_client` WSS 硬编码 `danger_accept_invalid_certs(true)`，无 opt-out                 | **P0** | [tardis/src/web/ws_client.rs](tardis/src/web/ws_client.rs#L80)                                                                                                    |
| S-03 | `TardisRsaKey::new_private_key(bits)` 不校验最小密钥长度                               | **P1** | [tardis/src/crypto/crypto_rsa.rs](tardis/src/crypto/crypto_rsa.rs)                                                                                                |
| S-04 | RSA 默认走 PKCS#1 v1.5，未提供 OAEP 变体                                               | **P1** | 同上                                                                                                                                                              |
| S-05 | SM4 `encrypt_cbc` 由调用方传 IV，未提供"随机 IV"安全便捷方法                           | **P1** | [tardis/src/crypto/crypto_sm2_4.rs](tardis/src/crypto/crypto_sm2_4.rs)                                                                                            |
| S-06 | `ComponentStore` 所有读写锁 `.expect("shouldn't poisoned")`，Poison 即崩               | **P1** | [tardis/src/basic/component.rs](tardis/src/basic/component.rs#L66) 等                                                                                             |
| S-07 | `ws_client` 无消息大小 / 帧大小上限，外部可触发 OOM                                    | **P1** | [tardis/src/web/ws_client.rs](tardis/src/web/ws_client.rs)                                                                                                        |
| S-08 | `ConfCenterConfig.url` 未校验 scheme，`conf-remote` 场景存在 SSRF / file-disclosure 面 | **P2** | [tardis/src/config/config_dto.rs](tardis/src/config/config_dto.rs#L192)                                                                                           |
| S-09 | `ComponentStore` 的 `unsafe impl Send/Sync` 安全论证不完整                             | **P2** | [tardis/src/basic/component.rs](tardis/src/basic/component.rs#L59-L62)                                                                                            |
| S-10 | 无 `verify_hmac` 常时比较 API，调用方易误用 `==`                                       | **P3** | [tardis/src/crypto/crypto_digest.rs](tardis/src/crypto/crypto_digest.rs)                                                                                          |
| S-11 | `rust-s3` fork 与 `poem` 的 git 依赖未 pin 到完整 40 位 SHA（poem rev 为 7 位短哈希）  | **P2** | [tardis/Cargo.toml](tardis/Cargo.toml)                                                                                                                            |
| S-12 | 公开 API 仍暴露 `encrypt_ecb` / `decrypt_ecb`，无 `#[deprecated]` 标记（仅 doc 警告）  | **P3** | [tardis/src/crypto/crypto_aead.rs](tardis/src/crypto/crypto_aead.rs)                                                                                              |

### 1.2 关键发现详述

#### S-01【P0】`ConfCenterConfig` 重名遮蔽导致密码进 debug 日志

- **事实链**：
  - `tardis/src/config/config_dto.rs` 第 10 行 `pub use component::*;` 把 `component::ConfCenterConfig`（**手写 Debug + `password.redact()`**）re-export 到 `config_dto` 命名空间；
  - 同文件第 **191 行**又定义了一个本地 `pub struct ConfCenterConfig`，**`#[derive(Debug, ...)]` + 明文 `pub password: String`**；
  - Rust glob `pub use` 的名字会被同作用域的本地定义**静默遮蔽**（无冲突错）。因此 `FrameworkConfig { conf_center: Option<ConfCenterConfig> }` 指向的是本地明文版本；
  - `tardis/src/config/config_processor.rs:63` 调用：
    ```rust
    debug!("=====[Tardis.Config] Content=====\n{:#?}\n=====", &config.fw);
    ```
    在 `conf-remote` 启用时会把 Nacos 登录密码以明文写入 debug 日志。
- **风险**：CWE-532（敏感信息写入日志）+ 运维在排错时极易把 debug 日志贴到 Issue/聊天。其他 `DBModuleConfig` / `CacheModuleConfig` / `MQModuleConfig` / `OSModuleConfig` / `MailModuleConfig` 均已手写 Debug redact，仅此处遗漏。
- **建议修复**：
  1. **立即**：删除 `config_dto.rs` 第 189–224 行的本地重复定义，复用 `component::ConfCenterConfig`（带 redact 的那一个）；
  2. 或保留本地定义，但为它手写 `impl Debug` 以 redact `password` 和 `username`；
  3. 顺手建议：把 `config_processor.rs:63` 的 `{:#?}` 换成只打印 `app.id` / `app.version` / feature 开关摘要，避免未来再次踩坑。

#### S-02【P0】`ws_client` 硬编码 `danger_accept_invalid_certs(true)`

- 位置：`tardis/src/web/ws_client.rs:80`
  ```rust
  Some(Connector::NativeTls(TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| { ... })?))
  ```
- 首轮报告已列为 P0-deferred，本轮再次确认。与 `web_client` / `mail_client` 的策略不一致（两者已 opt-in 化）。
- **风险**：任何 `wss://` 连接可被 MITM；`cluster` feature 依赖 `ws-client`，影响集群内部心跳与广播。
- **建议修复**：为 `WSClient::connect` 增加 `allow_invalid_certs: bool` 参数（默认 `false`）或为其所在 config DTO 新增字段，与 `web_client_dto::allow_invalid_certs` 一致；启用 opt-in 时打印 `warn!`。

#### S-03 & S-04【P1】RSA 默认 PKCS#1 v1.5 + 无最小密钥长度校验

- `tardis/src/crypto/crypto_rsa.rs` 中 `encrypt` / `decrypt` 均使用 `rsa::Pkcs1v15Encrypt`（Bleichenbacher 类侧信道）。
- `new_private_key(bits: usize)` 直接把 `bits` 交给 `RsaPrivateKey::new`，允许 512 / 1024 bit 等弱密钥。
- **建议**：
  1. 新增 `encrypt_oaep` / `decrypt_oaep` 方法（`rsa::Oaep::new::<sha2::Sha256>()`），在文档中标记为**推荐**；原 PKCS#1 v1.5 方法保留并文档标注"仅用于互操作"；
  2. 在 `new_private_key` 入口做 `bits >= 2048` 硬校验，失败返回 `406-tardis-crypto-rsa-key-size-error`。

#### S-05【P1】SM4 CBC IV 的使用模式不安全

- `tardis/src/crypto/crypto_sm2_4.rs` 的 `encrypt_cbc(data, key, iv)` 完全把 IV 责任交给调用方，未在库内提供"随机 IV + 拼接返回"的安全封装。
- **风险**：调用方常犯的错：全 0 IV、复用 IV、把 IV 硬编码在配置里。同 key 下复用 IV 会破坏 CBC 语义。
- **建议**：新增 `encrypt_cbc_random_iv(data, key) -> (ciphertext_with_iv, iv)` 或直接让 ciphertext 前 16 字节为 IV（行业通用做法），解密侧自动切片；原有接口保留但加 `# Warning`。

#### S-06【P1】`ComponentStore` 锁 poisoned 即 panic

- `tardis/src/basic/component.rs` 第 66、81、97、116 行均使用 `.expect("shouldn't poisoned")`。
- **风险**：`TardisComponentMap` 任一写入 panic 会导致之后每次读都 panic（连锁崩溃）。而 `TardisFuns` 是单例，这些锁贯穿整个运行时。
- **建议**：改为 `.unwrap_or_else(|e| e.into_inner())`（读写 map 只是 `HashMap` 的清空/插入，不会留下损坏不变式），或切换到 `parking_lot::RwLock`（无 poison）。

#### S-07【P1】`ws_client` 无消息/帧大小上限

- `tardis/src/web/ws_client.rs` 使用 `tokio_tungstenite::connect_async_tls_with_config`，未传 `WebSocketConfig`，消息和帧大小走默认（通常无上限或非常大）。
- **风险**：恶意对端可发超大帧让客户端一次性分配 GB 级内存（CWE-400）。
- **建议**：
  ```rust
  let ws_config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
      max_message_size: Some(10 * 1024 * 1024),   // 10 MiB
      max_frame_size:   Some(1 * 1024 * 1024),    // 1 MiB
      ..Default::default()
  };
  connect_async_tls_with_config(url, Some(ws_config), false, connector).await
  ```
  阈值通过 config DTO 暴露以允许调大。

#### S-08【P2】`conf-remote` URL scheme 未校验

- `ConfCenterConfig.url` 当前声明为 `String`（本地版本）/ `url::Url`（component 版本）。即便用了 `url::Url`，也没限制 scheme。若配置被污染为 `file:///etc/passwd`、`http://169.254.169.254/...` 等，`conf-remote` 的 reqwest 会直接请求。
- **建议**：在 `do_init` 开头 `assert!(matches!(u.scheme(), "http" | "https"))`，并对私网地址/SSRF 可选做 DNS 解析白名单（运维可控）。

#### S-09【P2】`ComponentStore` 的 `unsafe impl Send/Sync`

- `tardis/src/basic/component.rs:59-62` 直接 `unsafe impl Send/Sync for ComponentStore`，注释仅写 "It's actually Send, because the restriction."，没说明 "the restriction" 是什么。
- **现状其实是安全的**：所有插入路径都 `T: Any + Send + Sync + Clone`，但此处读者无法从 `ComponentStore` 的定义直接得出该不变式。
- **建议**：改写 safety comment 明确"所有插入路径都强制 `T: Send + Sync`，类型擦除后的 `Box<dyn Any + Send + Sync>` 是 Send+Sync 的"；或者直接把字段类型写成 `Box<dyn Any + Send + Sync>` 让 auto-trait 自动推导，删掉 `unsafe impl`。

#### S-10【P3】缺少常时 HMAC 比较 API

- 使用方常见误用：`computed_hmac == expected_hmac`（非常时比较，可能被 side channel 利用）。
- 库未提供 `verify_hmac_*` 系列便捷方法。
- **建议**：新增基于 `subtle::ConstantTimeEq` 的 `verify_hmac_sha256(data, key, expected_hex) -> TardisResult<bool>`。

#### S-11【P2】Git 依赖 pin 不严格

- `tardis/Cargo.toml`：`poem`/`poem-openapi`/`poem-openapi-derive`/`poem-grpc` 的 `rev = "99012c5"` 仅 7 位短哈希，git 重命名/重推历史有极小概率指向不同 commit。
- `rust-s3 = { git = "https://github.com/ZzIsGod1019/rust-s3.git", branch = "zz-obj-0103" }` 跟踪分支，任何后续 push 都会在 `cargo update` 时拉入。
- **建议**：全部改为完整 40 位 `rev = "..."`。`rust-s3` 尤其要 pin 到特定 commit；并把 "fork 存在的理由"写进 `tardis/Cargo.toml` 的注释。

#### S-12【P3】ECB 接口无 `#[deprecated]`

- [crypto_aead.rs](tardis/src/crypto/crypto_aead.rs) `encrypt_ecb` / `decrypt_ecb` 文档有 `# Warning`（本轮已修正），但缺少编译器可见的弃用提示。
- **建议**：
  ```rust
  #[deprecated(since = "0.1.0-rc.20", note = "ECB mode leaks plaintext structure; prefer AES-GCM")]
  pub fn encrypt_ecb(...) { ... }
  ```
  或把 ECB 放到 `crypto-legacy` feature。

---

## 2. 性能发现（Performance）

### 2.1 结论速览

| #    | 标题                                                                                      | 等级   | 位置                                                                               |
| ---- | ----------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| P-01 | `TardisFuns::init` 对独立组件（reldb/cache/mq/search/mail/os/web_client）顺序串行 `await` | **P1** | [tardis/src/lib.rs](tardis/src/lib.rs#L283-L340)                                   |
| P-02 | `*_by_module(code)` 每次请求 `code.to_lowercase()` 分配 `String`                          | **P1** | [tardis/src/lib.rs](tardis/src/lib.rs#L392) 等 7 处                                |
| P-03 | MQ `publish` / `request` 每次都 `create_channel` → 用完 `close`，没有通道池               | **P1** | [tardis/src/mq/mq_client.rs](tardis/src/mq/mq_client.rs#L50) × 3                   |
| P-04 | `search_client` 7 处 `unwrap_or(&String::new())`，每次错误路径都堆上分配空 `String`       | **P1** | [tardis/src/search/search_client.rs](tardis/src/search/search_client.rs#L117) 等   |
| P-05 | `ws_client` 出站使用 `mpsc::unbounded_channel`，无背压                                    | **P2** | [tardis/src/web/ws_client.rs](tardis/src/web/ws_client.rs#L107)                    |
| P-06 | `CachedJsonValue` 首次 deserialize 前 `self.json_value.clone()`                           | **P2** | [tardis/src/utils/cached_json_value.rs](tardis/src/utils/cached_json_value.rs)     |
| P-07 | `TardisLocale::init` 使用同步 `std::fs` + `BufReader::lines`，位于 async 启动路径         | **P2** | [tardis/src/basic/locale.rs](tardis/src/basic/locale.rs)                           |
| P-08 | `search_client::update` 通过 `format!` 拼接 ES script source（可读性+安全双差）           | **P2** | [tardis/src/search/search_client.rs](tardis/src/search/search_client.rs#L320)      |
| P-09 | `config_processor` 格式判断每次 `"toml".to_string()`                                      | **P3** | [tardis/src/config/config_processor.rs](tardis/src/config/config_processor.rs#L91) |
| P-10 | `web_server` TLS 启动时 `tls_cert.clone()` / `tls_key.clone()`（大 Vec）                  | **P3** | [tardis/src/web/web_server.rs](tardis/src/web/web_server.rs)                       |

### 2.2 关键发现详述

#### P-01【P1】冷启动/热加载组件初始化串行

- `TardisFuns::init` / `hot_reload` 对 `reldb`、`cache`、`mq`、`search`、`mail`、`os`、`web_client` 依次 `await?`。
- 这些组件**互不依赖**，每个 init 往往涉及网络握手（Redis ping、AMQP 建连、S3 head、SMTP 探测），串行等待≈ 各组件 RTT 之和。
- **实测影响（保守估计）**：7 个组件、每个 50–200ms → 串行 0.35–1.4s，并行后上限趋于最慢的单个组件（例如 AMQP 500ms），启动时间可下降 50–80%。
- **建议修复**：
  ```rust
  use futures::future::try_join_all;
  let mut futs: Vec<std::pin::Pin<Box<dyn Future<Output = TardisResult<()>> + Send>>> = Vec::new();
  #[cfg(feature = "cache")]
  if let Some(cache_config) = &fw_conf.cache { futs.push(Box::pin(async { tardis_instance().cache.init_by(cache_config).await.map(|_| ()) })); }
  // ... 其它组件
  try_join_all(futs).await?;
  ```
  `reldb` 与 `web_server` 存在相互引用时需保留部分顺序；其它独立的可放进 `join_all`。

#### P-02【P1】`*_by_module(code)` 每请求分配

- `tardis/src/lib.rs` 里 `cache_by_module` / `reldb_by_module` / `web_client_by_module` 等都以：
  ```rust
  let code = code.to_lowercase();
  let code = code.as_str();
  tardis_instance().cache.get(code).unwrap_or_else(Self::cache)
  ```
  开头（共 7 处）。
- **问题**：这些函数在 web handler 中每请求都会被调用，`to_lowercase` 即使对空字符串也会触发一次堆分配 + Unicode 判断。
- **建议**：
  - 方案 A（无破坏性）：在 `TardisComponentMap` 的 `insert` 时已统一小写化存储，此时 `get` 允许 `code: &str` + 内联 ASCII 小写比较，零分配；
  - 方案 B：提供 `*_by_module_ci(code: &str)` 使用 `unicase::Ascii<&str>` 作为 `HashMap` 键；
  - 方案 C（简单）：只有当 `code` 中确实含有大写字符时才分配（`if code.bytes().any(|b| b.is_ascii_uppercase())`），否则零拷贝走原路径。

#### P-03【P1】MQ 通道每次重建

- `tardis/src/mq/mq_client.rs` 的 `publish` / `request` / `response` 三处：
  ```rust
  let channel = self.con.create_channel().await?;
  // ... basic_publish ...
  channel.close(200u16, "").await?;
  ```
- **问题**：AMQP channel 轻量但 open/close 仍是 2 个 RTT；高吞吐场景（每秒上千条消息）下会变瓶颈；且 `close` 失败会被吞。
- **建议**：缓存一个（或少量）长连通道，或采用 `lapin::Channel` 的 clone（内部引用计数）模式。伪代码：
  ```rust
  pub struct TardisMQClient {
      con: Connection,
      default_channel: tokio::sync::RwLock<Option<Channel>>,
      ...
  }
  async fn get_or_create_channel(&self) -> TardisResult<Channel> {
      // fast path: read lock + check channel.status().connected()
      // slow path: write lock + create_channel
  }
  ```
  订阅 / `basic_consume` 逻辑保留独立 channel 以免互相阻塞。

#### P-04【P1】`search_client` 错误路径每次堆分配空 String

- 7 处模式：
  ```rust
  resp.body.as_ref().unwrap_or(&String::new())
  ```
- `&String::new()` 每次都在堆上开辟 0 字节缓冲（仍有栈对象+少量代码体积），更关键的是这是**无意义的默认值构造**。
- **修复**（机械替换，零行为风险）：
  ```rust
  resp.body.as_deref().unwrap_or("")
  ```
  建议集中一次性替换 7 处。

#### P-05【P2】`ws_client` 出站 `unbounded_channel`

- [ws_client.rs#L107](tardis/src/web/ws_client.rs#L107) 使用 `mpsc::unbounded_channel::<Message>()`。
- **风险**：生产者（应用代码）速度 > 下游 WebSocket 发送速度时，队列无上限堆积 → OOM。与安全 S-07 形成合力。
- **建议**：换成 `mpsc::channel(capacity)`，capacity 通过 config 暴露（默认 1024）。

#### P-06【P2】`CachedJsonValue` 首次 deserialize 前 clone

- `tardis/src/utils/cached_json_value.rs` 在首次按类型取值时 `Arc::new(serde_json::from_value(self.json_value.clone())?)`。
- **问题**：`serde_json::from_value(V)` 吃 Value 的所有权，但这里为了保留原始 Value 做了整棵 JSON 树的深拷贝。
- **建议**：改用 `serde_json::from_value(&self.json_value)`（有引用版），或提供 `deserialize::<T>()` 方法只缓存结果不保留原 Value。

#### P-07【P2】`TardisLocale::init` 同步 I/O

- `basic/locale.rs` 中 `File::open` + `BufReader::lines` 直接在 `init` 入口（异步上下文）同步读文件，包括热加载路径。
- **风险**：启动期间尚可接受；热加载时会阻塞整个 tokio runtime 当前 worker。
- **建议**：改 `tokio::fs` + `AsyncBufReadExt::lines()`；或 `tokio::task::spawn_blocking(|| { ... })` 封装。

#### P-08【P2】`search_client::update` 字符串拼 ES script（同时是 DSL 注入面）

- 见 [search_client.rs#L320](tardis/src/search/search_client.rs#L320) 附近 `format!(r#"{{ "script": {{ "source": "{source}", "params":{{{params}}} }} }}"#)`。
- **性能**：O(n) 多次 `format!`；**安全**：`key` / `value` 含 `"`、`\` 会破坏结构（与上一轮修掉的 `multi_search` 同族问题）。
- **建议**：用 `serde_json::json!({ "script": { "source": source, "params": params_obj } }).to_string()`，将 `params` 构造为 `serde_json::Map`。

#### P-09 / P-10【P3】零散分配

- 配置格式判断 `unwrap_or(&"toml".to_string())`：改 `unwrap_or(&"toml".into())` 或用 `as_deref().unwrap_or("toml")`；
- `web_server` TLS 启动时 `tls_cert.clone()` / `tls_key.clone()`：把字段类型换成 `Arc<Vec<u8>>`（只改库内部，无破坏性）。

---

## 3. 变更汇总（Change Summary）

| 分类     | 等级 | 数量   |
| -------- | ---- | ------ |
| 安全     | P0   | 2      |
| 安全     | P1   | 5      |
| 安全     | P2   | 3      |
| 安全     | P3   | 2      |
| 性能     | P1   | 4      |
| 性能     | P2   | 4      |
| 性能     | P3   | 2      |
| **合计** |      | **22** |

### 建议落地顺序

1. **立即**（同一 PR 内）：S-01（删除重复 `ConfCenterConfig`）、P-04（机械替换 7 处 `&String::new()` → `""`）。两者都是改动极小、风险极低、收益显著。
2. **近期**（独立 PR）：S-02（WS TLS opt-in 化）、S-03 + S-04（RSA 最小长度 + OAEP）、S-06（组件锁 poison 处理）、S-07 + P-05（WS 大小限制 + 背压）、P-01（启动并行化）、P-03（MQ 通道复用）、P-02（`*_by_module` 零分配）。
3. **规划**：S-05（SM4 随机 IV 封装）、S-08（conf-remote URL 白名单）、S-11（Cargo git 依赖 pin）、S-09 / S-12、其余 P2/P3 合并跟进。

### 合规确认（本轮复核通过）

- ✅ 上一轮 10 项已修复代码在本轮 `cargo check` / `clippy` 下仍然绿。
- ✅ 凭据脱敏对 `DBModuleConfig` / `CacheModuleConfig` / `MQModuleConfig` / `MailModuleConfig` / `OSModuleConfig` 有效（手写 Debug + `redact()`），仅 `ConfCenterConfig` 被本地重名遮蔽版本绕过 → 即本报告 S-01。
- ✅ 未在主线代码中发现其它 `unsafe` 使用点（除 `basic/component.rs:44,59,62` 三处，均已在 S-08/S-09 覆盖）。

---

## 4. 已核验剔除

首轮子智能体探查中，以下两项在原文描述可能误导，**本轮复核后调整或删除**：

- ❌ "整份 `FrameworkConfig` debug 日志泄密（P0）" → **部分不成立**。DB/cache/MQ/mail/OS 都已 redact。真正仍会泄露的只有 `ConfCenterConfig`（即 S-01），已更精确地定位为"重名遮蔽 bug"。
- ❌ "unsafe transmute TypeId → [u8;16] 无 size assert（P1）" → **影响有限**。标准库已保证 `TypeId` 布局为 `(u64,u64)` 16 字节；加 `const _: () = assert!(size_of::<TypeId>() == 16);` 纯属 defensive，不提升到 P1。合并到 S-09 语义下的 "unsafe 注释补强" 建议中。

---

## 附：可观测性 / 测试 建议（对应 §9 / §10）

- **观测**：为 S-02 / S-07 启用时统一打印 `warn!` 行（参考 `web_client` / `mail_client` 已有做法），方便 Ops 在运行时识别"处于不安全/受限模式"。
- **测试**：建议补充下列集成测试（均落入 `tardis/tests/`）：
  - `test_config_remote_debug_redaction.rs`：构造 `FrameworkConfig { conf_center: Some(... password: "s3cret") }`，`format!("{:#?}", &config.fw)` 结果断言 `!contains("s3cret")`。
  - `test_ws_client_tls_strict.rs`：连自签 `wss://` 断言失败（默认严格）。
  - `test_crypto_rsa_min_key.rs`：`new_private_key(1024)` 应返回 `Err(...)`。
  - `test_search_update_injection.rs`：`update` 的 value 含 `"` / `\` 不破坏 JSON。

