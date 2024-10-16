
**Elegant, Clean Rust development framework🛸**

---

[![Crate](https://img.shields.io/crates/v/tardis.svg)](https://crates.io/crates/tardis)
[![Docs](https://docs.rs/tardis/badge.svg)](https://docs.rs/tardis)
[![Build Status](https://github.com/ideal-world/tardis/actions/workflows/cicd.yml/badge.svg)](https://github.com/ideal-world/tardis/actions/workflows/cicd.yml)
[![Test Coverage](https://codecov.io/gh/ideal-world/tardis/branch/main/graph/badge.svg?token=L1LQ8DLUS2)](https://codecov.io/gh/ideal-world/tardis)
[![License](https://img.shields.io/github/license/ideal-world/tardis)](https://github.com/ideal-world/tardis/blob/main/LICENSE)

> TARDIS(\[tɑːrdɪs\] "Time And Relative Dimension In Space") From "Doctor Who".

## 💖 Core functions

* Relational database client for MySQL, PostgresSQL
* Web service and web client for OpenAPI v3.x
* Distributed cache client for Redis protocol
* RabbitMQ client for AMQP protocol
* Search client for Elasticsearch
* Mail client for SMTP protocol
* Object Storage client for arbitrary S3 compatible APIs
* Mainstream encryption algorithms and SM2/3/4 algorithms
* Containerized unit testing of mainstream middleware
* Multi-environment configuration
* Multi-application aggregation
* Configure encryption support
* Internationalization and localization support
* Commonly used operations (E.g. uniform error handling, encryption and decryption, regular checksums)

## ⚙️Key Features

* ``conf-remote`` enable the unified configuration center
* ``crypto`` encryption, decryption and digest operations
* ``crypto-with-sm`` encryption, decryption and digest with SM.x operations
* ``future`` asynchronous operations
* ``reldb-core`` relational database core operations(based on [SeaORM](https://github.com/SeaQL/sea-orm))
* ``reldb-postgres`` relational database with postgres driver
* ``reldb-mysql`` relational database with mysql driver
* ``reldb-sqlite`` relational database with sqlite driver
* ``reldb`` relational database with postgres/mysql/sqlite drivers
* ``web-server`` web service operations(based on [Poem](https://github.com/poem-web/poem))
* ``web-server-grpc`` grpc web service based on [Poem](https://github.com/poem-web/poem)
* ``web-client`` web client operations
* ``ws-client`` websocket client operations
* ``cache`` cache operations
* ``mq`` message queue operations
* ``mail`` mail send operations
* ``os`` object Storage operations
* ``test`` unit test operations
* ``tracing`` open telemetry support
* ``tokio-console`` console subscriber layer supported by [tokio-console](https://github.com/tokio-rs/console)
* ``tracing-appender`` write log into file periodically.
* ``build-info`` get build info like package version or git version

## 🚀 Quick start

The core operations of the framework all use ``TardisFuns`` as an entry point.
E.g.

```rust ignore
TardisFuns::init(relative_path)      // Initialize the configuration
TardisFuns::field.x                  // Some field operations
TardisFuns::reldb().x                // Some relational database operations
TardisFuns::web_server().x           // Some web service operations
```

### Web service example

Dependency Configuration

```toml
[dependencies]
tardis = { version = "^0", features = ["web-server"] }
```

Processor Configuration

```rust ignore
use tardis::basic::error::TardisError;
use tardis::web::poem_openapi;
use tardis::web::poem_openapi::param::Query;
use tardis::web::web_resp::{TardisApiResult, TardisResp};

pub struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> TardisResult<String> {
        match name.0 {
            Some(name) => TardisResp::ok(format!("hello, {name}!")),
            None => TardisResp::err(TardisError::NotFound("name does not exist".to_string())),
        }
    }
}
```

Startup class configuration

```rust ignore
use tardis::basic::result::TardisResult;
use tardis::tokio;
use tardis::TardisFuns;
use crate::processor::Api;
mod processor;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Initial configuration
    TardisFuns::init("config").await?;
    // Register the processor and start the web service
    TardisFuns::web_server().add_module("", Api).start().await;
    TardisFuns::web_server().web_server.await;
    Ok(())
}
```

### Dependencies

In order to use gRPC features, you need the `protoc` Protocol Buffers compiler, along with Protocol Buffers resource files.

#### Ubuntu

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y protobuf-compiler libprotobuf-dev
```

#### Alpine Linux

```sh
sudo apk add protoc protobuf-dev
```

#### macOS

Assuming [Homebrew](https://brew.sh/) is already installed. (If not, see instructions for installing Homebrew on [the Homebrew website](https://brew.sh/).)

```zsh
brew install protobuf
```

#### Windows

- Download the latest version of `protoc-xx.y-win64.zip` from [HERE](https://github.com/protocolbuffers/protobuf/releases/latest)
- Extract the file `bin\protoc.exe` and put it somewhere in the `PATH`
- Verify installation by opening a command prompt and enter `protoc --version`

### More examples

```plaintext ignore
|-- examples
  |-- reldb              Relational database usage example
  |-- web-basic          Web service Usage Example
  |-- web-client         Web client Usage Example
  |-- websocket          WebSocket Usage Example
  |-- cache              Cache Usage Example
  |-- mq                 Message Queue Usage Example
  |-- todos              A complete project usage example
  |-- multi-apps         Multi-application aggregation example
  |-- pg-graph-search    Graph search by Postgresql example
  |-- perf-test          Performance test case
  |-- tracing-otlp       Trace and send data by otlp prococol example
```

### FAQ

* An `` failed to run custom build command for openssl-sys`` error occurs when running under Windows.The solution is as follows( @see <https://github.com/sfackler/rust-openssl/issues/1062> ):

  ```shell
  git clone https://github.com/Microsoft/vcpkg --depth=1
  cd vcpkg
  bootstrap-vcpkg.bat
  vcpkg.exe integrate install
  vcpkg.exe install openssl:x64-windows-static
  set OPENSSL_NO_VENDOR=1
  set OPENSSL_DIR=<Current Dir>\packages\openssl_x64-windows-static
  ```
* An `` failed to run custom build command for openssl-sys`` error occurs when running under Ubuntu(similar to other distributions):

  ```shell
  apt install build-essential perl pkg-config libssl-dev
  ```
* FreeBSD deployment for ``openssl-sys``

  ```shell
  sudo pkg install cmake ninja zip pkgconf gmake
  git clone https://github.com/Microsoft/vcpkg --depth=1
  cd vcpkg
  sh bootstrap-vcpkg.sh
  ./vcpkg integrate install
  ./vcpkg install openssl
  ```
* An `` failed to run custom build command for opentelemetry-proto`` error occurs when running under Linux:

  ```shell
  apt install protobuf-compiler
  ```
* An `` failed to run custom build command for opentelemetry-proto`` error occurs when running under MacOS:

  ```shell
  brew install protobuf
  ```

---

Thanks to `Jetbrains` for the [Open Source License](https://www.jetbrains.com/community/opensource/)
