// https://github.com/poem-web/poem

extern crate core;

use std::env;
use std::str::FromStr;
use std::time::Duration;

use poem::endpoint::{BoxEndpoint, ToDynEndpoint};
use poem::http::Method;
use poem::{IntoResponse, Middleware, Response};
use serde_json::json;
use tardis::basic::tracing::TardisTracing;
use tardis::web::web_server::WebServerModule;
use tokio::time::sleep;
use tracing::info;

use tardis::basic::dto::TardisContext;
use tardis::basic::error::TardisError;
use tardis::basic::field::TrimString;
use tardis::basic::result::{TardisResult, TARDIS_RESULT_ACCEPTED_CODE, TARDIS_RESULT_SUCCESS_CODE};
use tardis::config::config_dto::{CacheModuleConfig, FrameworkConfig, LogConfig, TardisConfig, WebClientConfig, WebServerCommonConfig, WebServerConfig, WebServerModuleConfig};
use tardis::serde::{Deserialize, Serialize};
use tardis::test::test_container::TardisTestContainer;
use tardis::web::context_extractor::{TardisContextExtractor, TOKEN_FLAG};
use tardis::web::poem::{Endpoint, Request};
use tardis::web::poem_openapi::{param::Path, payload::Json, Object, OpenApi, Tags};
use tardis::web::web_resp::{TardisApiResult, TardisResp};
use tardis::TardisFuns;

#[allow(non_snake_case)]
mod helloworld_grpc {
    include!("./grpc/rust/helloworld.rs");
}
pub use helloworld_grpc::*;
use poem_grpc::{Request as GrpcRequest, Response as GrpcResponse, Status as GrpcStatus};
use tracing_subscriber::filter::Directive;

const TLS_KEY: &str = r#"
-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCq4QyODxghypMz
u3wSNYAH7qsekRasbkFWlzRlfCkVfxMynGh8uRfNod4UrHAWTXDJAneHoFgqXemI
Vf1z87r0T2NFf3+oochAKvE6z9hNSeBbLIeZXPQQcabtO9T5cS4anK3k0mbgRpdM
bDhv4p+fbwCp6gOA/qeqs58p/f5UoXYVcjKmSQQAA5KSm4bPbBl89fiUFv1Rcp5H
L+sMIkFpwq8lYnEqKvV/1V8isMvhVDUo7MrIUwgqn6+n7KJIIsugOqxVKrYA4E6R
oFC1V3O5jdTrDvWhzlHel5VWHX6rG/4J4GvD+6nMZ2CvgK8DeW/vEsRUlPDPoTc0
QRftHEO3AgMBAAECggEAGJC42tVNrVHvfoDl8cIyMTG89Ox7u3cwOnALTsmeKGJ3
0o9MsB110BCEmik+Bd7FJ4iMwXkqI5ETqQ9fm+M+ja+6ADw3kHkNjjf+LOvDVz0X
HVRV/BSyW4jTLAitcdy0+YtrrlkXBAfx6UEnjeIg+20cRdEIBuvVE8O1znYesXC2
oALXl2f1lJE8n83rijn5ecxuJIM3fKt/tgfMPEfyz7OqY3USKGzaDUZyScLW1b0V
D2m1os7lzEuIkqK6JuunZpx36EEBTyWFFwnU/sobyL0h70qTXrtjZoO5qgOTS9rd
CkKAlXYessSTsH8L0jSZKMMp3Ho6y91b9VFxyfU6LQKBgQDMlDSuQhFqrNHcmtN0
2adrJ15gJRJZzvsChaZd5PPVV4a55QAm5OOo5eA+zX/9/qHPjlgWigbDyyz+Nvx4
Bl55XDJpkErrcdAgLyd81Dtzd5BQI5cSC/wSZdXJjw7RngmEfa9IscpiG8vs0MZJ
Tw56JjY1ue4vMwKOP4mndQ3GWwKBgQDV1GaZKFsq1GpDjzyIetedQ1JV/IGIygZY
ekPjla825EmDq23A5zkFYWK8aoRh44OnMmj7sP1UcJWgTgAiYvnowAVrvPuudX/F
DR0UsdBaOmJqsjuy6Xn9cldN85zbqpVS61+6OrNEGzHx//5i5jLr9zw2XAt9GGWo
fTsVkWvO1QKBgF9dPul0VtYZVYK2kZfI1ig3I+FBprpCp/PXBWSDk76BnIYPX/DB
hfZ3of7koKNwDVHJkvp+wQSIM6MVUr9IiMWd2somvyXd2h0Gniusa0I6HAWfcY6y
E4EoA25/x3KjbuBaDlmety7gskDkWWpW9fKu2VpWH9fUuX5B1BNBl3g9AoGBANVD
5qBS07q/6MxBDArDGlFLV89S7I7Vj8anCxbtr7d7sKaWT/zZoNFw890gD7DiDeiw
Kmz9dWzGbTVZFmE1fjNZcQ6nig3SOwD5t0twnXGgUZBA+7HRk03owJKKqqOcWxo8
j1laOnlu9j17KOjS127pQzCkVQELWDjXzhoQ1AmRAoGACFk2lgato0S/PTdHhsOa
0G4zCbIpjndRYuW5IMURpiGeQEZ5unIuX72lx180ncj+PTw6DxxiEsESDhIp1VfW
RWf7YsEUgQICLka42SY+UsSfEe7Wya3ZM4bhc+wVi2rgjVBriuBC5UzMAWpCyMri
7A2laBCqWVkgks5BQPLtlXg=
-----END PRIVATE KEY-----
"#;

const TLS_CERT: &str = r#"
-----BEGIN CERTIFICATE-----
MIICrzCCAZcCFBAFc1XYPWC+wosehbOnnxfi0t2KMA0GCSqGSIb3DQEBCwUAMBQx
EjAQBgNVBAMMCWxvY2FsaG9zdDAeFw0yNDAyMjgxMDAwMTdaFw0yNTAyMjcxMDAw
MTdaMBQxEjAQBgNVBAMMCWxvY2FsaG9zdDCCASIwDQYJKoZIhvcNAQEBBQADggEP
ADCCAQoCggEBAKrhDI4PGCHKkzO7fBI1gAfuqx6RFqxuQVaXNGV8KRV/EzKcaHy5
F82h3hSscBZNcMkCd4egWCpd6YhV/XPzuvRPY0V/f6ihyEAq8TrP2E1J4Fssh5lc
9BBxpu071PlxLhqcreTSZuBGl0xsOG/in59vAKnqA4D+p6qznyn9/lShdhVyMqZJ
BAADkpKbhs9sGXz1+JQW/VFynkcv6wwiQWnCryVicSoq9X/VXyKwy+FUNSjsyshT
CCqfr6fsokgiy6A6rFUqtgDgTpGgULVXc7mN1OsO9aHOUd6XlVYdfqsb/gnga8P7
qcxnYK+ArwN5b+8SxFSU8M+hNzRBF+0cQ7cCAwEAATANBgkqhkiG9w0BAQsFAAOC
AQEAXWK8bSNLcmnHByh0gt+i2tuH4luSopz95Sj2a2rbVVcnKUTy5vzhRgSc0uMr
dCoOB67X2vDfN7DU3ZGUEjgVA3mwntW19Vv03DBvZBsYY9uzZdv8NXDSRRiKNbU4
dXS3HhsPFdbgx1zjmbjOU5/JEkw4d6Ijcy09mCqiaJd1IVLCKvvAfOXkAG91iWpQ
ZDloEhXbOC4/jzxi9cvNWIOf/DpqdcMMXAMOd92ubmuYV5YhusvCL/9rv9cQsm7q
bY588beOczzrXB0ldJAHZkoQFccSM1sP7pmUqgBOR0ZedmMzR37GuKjEpc/TvXHR
5TSjY7LCQ8H807/6Fil5WUDSZg==
-----END CERTIFICATE-----
"#;

#[tokio::test(flavor = "multi_thread")]
async fn test_web_server() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace,poem_grpc=trace,poem=trace");
    TardisTracing::initializer().with_env_layer().with_fmt_layer().with_opentelemetry_layer().init();
    let web_url = "https://localhost:8080";

    let redis_container = TardisTestContainer::redis_custom().await?;
    let redis_port = redis_container.get_host_port_ipv4(6379).await?;
    let redis_url = format!("redis://127.0.0.1:{redis_port}/0");
    start_serv(web_url, &redis_url).await?;
    sleep(Duration::from_millis(500)).await;

    test_basic(web_url).await?;
    test_validate(web_url).await?;
    test_context(web_url).await?;
    test_security().await?;
    test_middleware().await?;
    TardisFuns::shutdown().await?;

    Ok(())
}

async fn start_serv(web_url: &str, redis_url: &str) -> TardisResult<()> {
    let fw_config = FrameworkConfig::builder()
        .web_server(
            WebServerConfig::builder()
                .common(WebServerCommonConfig::builder().port(8080).tls_key(TLS_KEY).tls_cert(TLS_CERT).security_hide_err_msg(false).build())
                .modules([
                    (
                        "todo".to_string(),
                        WebServerModuleConfig::builder()
                            .name("todo_app")
                            .doc_urls([("test env".to_string(), web_url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())])
                            .build(),
                    ),
                    ("other".to_string(), WebServerModuleConfig::builder().name("other app").build()),
                    ("grpc".to_string(), WebServerModuleConfig::builder().name("grpc app").build()),
                ])
                .default(Default::default())
                .build(),
        )
        .cache(CacheModuleConfig::builder().url(redis_url.parse().expect("invalid redis url")).build())
        .log(
            LogConfig::builder()
                .directives([Directive::from_str("poem=debug").expect("invalid directives")])
                .level(Directive::from_str("info").expect("invalid directives"))
                .build(),
        )
        .build();
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: fw_config.clone(),
    })
    .await?;
    TardisFuns::web_server()
        .add_module("todo", TodosApi)
        .await
        .add_module("other", OtherApi)
        .await
        .add_grpc_module("grpc", GreeterServer::new(GreeterGrpcService))
        .await
        .start()
        .await?;
    // TardisFuns::web_server().shutdown().await?;
    Ok(())
}

async fn test_basic(url: &str) -> TardisResult<()> {
    // Normal
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1").as_str(), None).await?;
    assert_eq!(response.code, 200);
    assert_eq!(response.body.as_ref().unwrap().code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.body.as_ref().unwrap().data.as_ref().unwrap().code.to_string(), "code1");

    // Accepted
    let response = TardisFuns::web_client().get::<TardisResp<String>>(format!("{url}/todo/todos/1/async").as_str(), None).await?;
    assert_eq!(response.code, 200);
    assert_eq!(response.body.as_ref().unwrap().code, TARDIS_RESULT_ACCEPTED_CODE);
    assert_eq!(response.body.as_ref().unwrap().data.as_ref().unwrap(), "/todos/1/status");

    // Business Error
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/err").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::conflict("异常", "").code);
    assert_eq!(response.msg, TardisError::conflict("异常", "").message);

    // Not Found
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/ss").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::not_found("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: not found");

    let grpc_client = GreeterClient::new(poem_grpc::ClientConfig::builder().uri("https://localhost:8080/grpc").build().unwrap());
    let _grpc_response = grpc_client.say_hello(GrpcRequest::new(HelloRequest { name: "Tardis".into() })).await;
    // "error trying to connect: invalid peer certificate: Expired" our certificate has expired
    // assert!(grpc_response.is_ok());
    // assert_eq!(grpc_response.unwrap().message, "Hello Tardis!");
    Ok(())
}

async fn test_validate(url: &str) -> TardisResult<()> {
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/ss").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: failed to parse path `param0`: failed to parse "integer_int64": invalid digit found in string"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: String::new(),
                eq: String::new(),
                range: 0,
                mail: String::new(),
                contain: String::new(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `len` verification failed. minLength(1)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: String::new(),
                range: 0,
                mail: String::new(),
                contain: String::new(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `eq` verification failed. minLength(5)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 0,
                mail: String::new(),
                contain: String::new(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `range` verification failed. minimum(1, exclusive: false)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss.ss".to_string(),
                contain: String::new(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `mail` verification failed. Invalid mail format"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: String::new(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `contain` verification failed. pattern(".*gmail.*")"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: String::new(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `phone` verification failed. Invalid phone number format"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: "18654110201".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `item_len` verification failed. minItems(1)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: "18654110201".to_string(),
                item_len: vec!["ddd".to_string()],
                item_unique: vec!["ddd".to_string(), "ddd".to_string()],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        r#"[Tardis.WebServer] Process error: parse request payload error: failed to parse "ValidateReq": field `item_unique` verification failed. uniqueItems()"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: "18654110201".to_string(),
                item_len: vec!["ddd".to_string()],
                item_unique: vec!["ddd1".to_string(), "ddd2".to_string()],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);

    Ok(())
}

async fn test_context(url: &str) -> TardisResult<()> {
    let response = TardisFuns::web_client().get::<TardisResp<String>>(format!("{url}/other/context_in_header").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::unauthorized("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: authorization error");
    let fw_config = TardisFuns::fw_config();
    let web_server_config = fw_config.web_server();
    // from header
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(web_server_config.context_conf.context_header_name.clone(), "sss".to_owned())],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::unauthorized("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: authorization error");

    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(web_server_config.context_conf.context_header_name.clone(), "c3Nz".to_owned())],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::unauthorized("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: authorization error");

    let context = TardisContext {
        own_paths: "tenant1/app1".to_string(),
        ak: "ak1".to_string(),
        roles: vec!["r1".to_string(), "管理员".to_string()],
        groups: vec!["g1".to_string()],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(
                web_server_config.context_conf.context_header_name.clone(),
                TardisFuns::json.obj_to_string(&context).unwrap(),
            )],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::unauthorized("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: authorization error");

    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(
                web_server_config.context_conf.context_header_name.clone(),
                TardisFuns::crypto.base64.encode(TardisFuns::json.obj_to_string(&context).unwrap()),
            )],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap(), "管理员");

    // from cache
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(web_server_config.context_conf.context_header_name.clone(), format!("{TOKEN_FLAG}token1"))],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::unauthorized("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: authorization error");

    let context = TardisContext {
        own_paths: "tenant1/app1".to_string(),
        ak: "ak1".to_string(),
        roles: vec!["r1".to_string(), "管理员".to_string()],
        groups: vec!["g1".to_string()],
        owner: "acc1".to_string(),
        ext: Default::default(),
        sync_task_fns: Default::default(),
        async_task_fns: Default::default(),
    };
    TardisFuns::cache()
        .set(
            format!("{}token1", web_server_config.context_conf.token_cache_key).as_str(),
            TardisFuns::json.obj_to_string(&context).unwrap().as_str(),
        )
        .await
        .unwrap();
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{url}/other/context_in_header").as_str(),
            [(web_server_config.context_conf.context_header_name.clone(), format!("{TOKEN_FLAG}token1"))],
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap(), "管理员");

    Ok(())
}

async fn test_security() -> TardisResult<()> {
    let url = "https://localhost:8081";
    TardisFuns::shutdown().await?;
    let fw_config = FrameworkConfig::builder()
        .web_client(WebClientConfig::default())
        .web_server(
            WebServerConfig::builder()
                .common(WebServerCommonConfig::builder().port(8081).tls_key(TLS_KEY).tls_cert(TLS_CERT).security_hide_err_msg(true).build())
                .modules([
                    (
                        "todo".to_string(),
                        WebServerModuleConfig::builder()
                            .name("todo_app")
                            .doc_urls([("test env".to_string(), url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())])
                            .build(),
                    ),
                    ("other".to_string(), WebServerModuleConfig::builder().name("other app").build()),
                ])
                .default(Default::default())
                .build(),
        )
        .build();
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: fw_config.clone(),
    })
    .await?;
    TardisFuns::web_server().add_module("todo", TodosApi).await.add_module("other", OtherApi).await.start().await?;

    sleep(Duration::from_millis(500)).await;

    // Normal
    let response = TardisFuns::web_client()
        .post::<TodoAddReq, TardisResp<String>>(
            format!("{url}/todo/todos").as_str(),
            &TodoAddReq {
                code: "  编码1 ".into(),
                description: "测试".to_string(),
                done: false,
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap(), "编码1");

    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap().description, "测试");

    // Business Error
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/err").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::conflict("异常", "").code);
    assert_eq!(
        response.msg,
        "[Tardis.WebServer] Security is enabled, detailed errors are hidden, please check the server logs"
    );

    // Not Found
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/ss").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::not_found("", "").code);
    assert_eq!(
        response.msg,
        "[Tardis.WebServer] Security is enabled, detailed errors are hidden, please check the server logs"
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{url}/other/validate").as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: "18654110201".to_string(),
                item_len: vec!["ddd".to_string()],
                item_unique: vec!["ddd".to_string(), "ddd".to_string()],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::bad_request("", "").code);
    assert_eq!(
        response.msg,
        "[Tardis.WebServer] Security is enabled, detailed errors are hidden, please check the server logs"
    );

    Ok(())
}

async fn test_middleware() -> TardisResult<()> {
    let url = "http://localhost:8082";
    TardisFuns::shutdown().await?;
    let fw_config = FrameworkConfig::builder()
        .web_server(
            WebServerConfig::builder()
                .common(WebServerCommonConfig::builder().port(8082).build())
                .modules([
                    (
                        "todo".to_string(),
                        WebServerModuleConfig::builder()
                            .name("todo_app")
                            .doc_urls([("test env".to_string(), url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())])
                            .build(),
                    ),
                    ("other".to_string(), WebServerModuleConfig::builder().name("other app").build()),
                ])
                .default(Default::default())
                .build(),
        )
        .build();
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: fw_config.clone(),
    })
    .await?;
    TardisFuns::web_server()
        .add_module("todo", WebServerModule::new(TodosApi).middleware((TodosApiMiddleware1, TodosApiMiddleware2)))
        .await
        .add_module("other", OtherApi)
        .await
        .start()
        .await?;

    // Normal
    let response = TardisFuns::web_client()
        .post::<TodoAddReq, TardisResp<String>>(
            format!("{url}/todo/todos").as_str(),
            &TodoAddReq {
                code: "编码2".into(),
                description: "测试".to_string(),
                done: false,
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap(), "编码2");

    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    let data = response.data.unwrap();
    assert_eq!(data.description, "exec TodosApiMWImpl");
    assert!(data.done);

    // Business Error
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/err").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::conflict("异常", "").code);
    assert_eq!(response.msg, TardisError::conflict("异常", "").message);

    // Not Found
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{url}/todo/todos/1/ss").as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::not_found("", "").code);
    assert_eq!(response.msg, "[Tardis.WebServer] Process error: not found");

    Ok(())
}

#[derive(Tags)]
enum FunTags {
    #[oai(rename = "Todo1测试")]
    Todo1,
}

#[derive(Object, Serialize, Deserialize, Debug)]
struct TodoResp {
    id: i64,
    code: TrimString,
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug, Clone)]
struct TodoAddReq {
    code: TrimString,
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug)]
struct TodoModifyReq {
    description: Option<String>,
    done: Option<bool>,
}

#[derive(Object, Serialize, Deserialize, Debug)]
struct ValidateReq {
    #[oai(validator(min_length = "1", max_length = "10"))]
    len: String,
    #[oai(validator(min_length = "5", max_length = "5"))]
    eq: String,
    #[oai(validator(minimum(value = "1", exclusive = "false"), maximum(value = "500", exclusive)))]
    range: u32,
    #[oai(validator(custom = "tardis::web::web_validation::Mail"))]
    mail: String,
    #[oai(validator(pattern = r".*gmail.*"))]
    contain: String,
    #[oai(validator(custom = "tardis::web::web_validation::Phone"))]
    phone: String,
    #[oai(validator(min_items = "1", max_items = "3"))]
    item_len: Vec<String>,
    #[oai(validator(unique_items))]
    item_unique: Vec<String>,
}
#[derive(Debug, Clone, Copy)]
struct TodosApi;

#[OpenApi(tag = "FunTags::Todo1")]
impl TodosApi {
    #[oai(path = "/todos", method = "post")]
    async fn create(&self, todo_add_req: Json<TodoAddReq>) -> TardisApiResult<String> {
        TardisResp::ok(todo_add_req.code.to_string())
    }

    #[oai(path = "/todos/:id", method = "get")]
    async fn get(&self, id: Path<i64>) -> TardisApiResult<TodoResp> {
        TardisResp::ok(TodoResp {
            id: id.0,
            code: "  code1  ".into(),
            description: "测试".to_string(),
            done: false,
        })
    }

    #[oai(path = "/todos/:id/async", method = "get")]
    async fn get_async(&self, id: Path<i64>) -> TardisApiResult<String> {
        TardisResp::accepted(format!("/todos/{}/status", id.0))
    }

    #[oai(path = "/todos/:_id/err", method = "get")]
    async fn get_by_error(&self, _id: Path<i64>) -> TardisApiResult<TodoResp> {
        TardisResp::err(TardisError::conflict("异常", ""))
    }
}
#[derive(Clone)]
struct OtherApi;

#[OpenApi]
impl OtherApi {
    #[oai(path = "/validate", method = "post")]
    async fn validate(&self, _req: Json<ValidateReq>) -> TardisApiResult<String> {
        TardisResp::ok("".into())
    }

    #[oai(path = "/context_in_header", method = "get")]
    async fn context_in_header(&self, ctx: TardisContextExtractor) -> TardisApiResult<String> {
        TardisResp::ok(ctx.0.roles.get(1).unwrap().to_string())
    }
}
#[derive(Clone, Debug)]
struct TodosApiMiddleware1;

impl Middleware<BoxEndpoint<'static>> for TodosApiMiddleware1 {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: BoxEndpoint<'static>) -> Self::Output {
        pub struct TodosApiMWImpl1<E>(E);

        impl<E: Endpoint> Endpoint for TodosApiMWImpl1<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> poem::Result<Self::Output> {
                let method = req.method().clone();
                let url = req.uri().clone();
                if method == Method::POST {
                    let req_body = req.take_body().into_json::<TodoAddReq>().await.expect("Test req take body error");
                    info!("Exec TodosApiMWImpl {} req{:?}", method, req_body);
                    req.set_body(json!({"code":req_body.code,"description":req_body.description,"done":req_body.done}).to_string());
                }
                match self.0.call(req).await {
                    Ok(resp) => {
                        let mut resp = resp.into_response();
                        match method {
                            Method::GET => {
                                let resp_body = resp.take_body().into_json::<TardisResp<TodoResp>>().await.expect("Test resp take body error");
                                info!("Exec TodosApiMWImpl {} resp{:?}", method, resp_body);
                                let resp_body_data = resp_body.data.unwrap();
                                resp.set_body(
                                    json!({
                                        "code": resp_body.code,
                                        "msg":resp_body.msg,
                                        "data":{
                                            "id": resp_body_data.id,
                                            "code": resp_body_data.code,
                                            "description": "exec TodosApiMWImpl",
                                            "done":resp_body_data.done
                                        }
                                    })
                                    .to_string(),
                                );
                            }
                            Method::POST => {
                                let resp_body = resp.take_body().into_json::<TardisResp<String>>().await.expect("Test resp take body error");
                                info!("Exec TodosApiMWImpl {} resp{:?}", method, resp_body);
                                let resp_body_data = resp_body.data.unwrap();
                                resp.set_body(
                                    json!({
                                        "code": resp_body.code,
                                        "msg":resp_body.msg,
                                        "data":resp_body_data
                                    })
                                    .to_string(),
                                );
                            }
                            _ => {}
                        }

                        Ok(resp)
                    }
                    Err(r) => {
                        info!("Exec TodosApiMWImpl Err {} url{:?}", method, url,);
                        Err(r)
                    }
                }
            }
        }

        Box::new(ToDynEndpoint(TodosApiMWImpl1(ep)))
    }
}

#[derive(Clone, Debug)]
struct TodosApiMiddleware2;

impl Middleware<BoxEndpoint<'static>> for TodosApiMiddleware2 {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: BoxEndpoint<'static>) -> Self::Output {
        pub struct TodosApiMWImpl2<E>(E);

        impl<E: Endpoint> Endpoint for TodosApiMWImpl2<E> {
            type Output = Response;

            async fn call(&self, mut req: Request) -> poem::Result<Self::Output> {
                let method = req.method().clone();
                let url = req.uri().clone();
                if method == Method::POST {
                    let req_body = req.take_body().into_json::<TodoAddReq>().await.expect("Test req take body error");
                    info!("Exec TodosApiMWImpl2 {} req{:?}", method, req_body);
                    req.set_body(json!({"code":req_body.code,"description":req_body.description,"done":req_body.done}).to_string());
                }
                match self.0.call(req).await {
                    Ok(resp) => {
                        let mut resp = resp.into_response();
                        match method {
                            Method::GET => {
                                let resp_body = resp.take_body().into_json::<TardisResp<TodoResp>>().await.expect("Test resp take body error");
                                info!("Exec TodosApiMWImpl2 {} resp{:?}", method, resp_body);
                                let resp_body_data = resp_body.data.unwrap();
                                resp.set_body(
                                    json!({
                                        "code": resp_body.code,
                                        "msg":resp_body.msg,
                                        "data":{
                                            "id": resp_body_data.id,
                                            "code": resp_body_data.code,
                                            "description": resp_body_data.description,
                                            "done":true
                                        }
                                    })
                                    .to_string(),
                                );
                            }
                            Method::POST => {
                                let resp_body = resp.take_body().into_json::<TardisResp<String>>().await.expect("Test resp take body error");
                                info!("Exec TodosApiMWImpl2 {} resp{:?}", method, resp_body);
                                let resp_body_data = resp_body.data.unwrap();
                                resp.set_body(
                                    json!({
                                        "code": resp_body.code,
                                        "msg":resp_body.msg,
                                        "data":resp_body_data
                                    })
                                    .to_string(),
                                );
                            }
                            _ => {}
                        }

                        Ok(resp)
                    }
                    Err(r) => {
                        info!("Exec TodosApiMWImpl2 Err {} url{:?}", method, url,);
                        Err(r)
                    }
                }
            }
        }

        Box::new(ToDynEndpoint(TodosApiMWImpl2(ep)))
    }
}

#[derive(Clone, Default)]
pub struct GreeterGrpcService;
impl Greeter for GreeterGrpcService {
    async fn say_hello(&self, request: GrpcRequest<HelloRequest>) -> Result<GrpcResponse<HelloReply>, GrpcStatus> {
        info!("GreeterGrpcService say_hello {:?}", request);
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(GrpcResponse::new(reply))
    }
}
