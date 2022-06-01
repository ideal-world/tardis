// https://github.com/poem-web/poem

extern crate core;

use std::collections::HashMap;
use std::time::Duration;

use testcontainers::clients;
use tokio::time::sleep;

use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig, WebServerModuleConfig};
use tardis::basic::dto::TardisContext;
use tardis::basic::error::TardisError;
use tardis::basic::field::TrimString;
use tardis::basic::result::{TardisResult, TARDIS_RESULT_SUCCESS_CODE};
use tardis::serde::{Deserialize, Serialize};
use tardis::test::test_container::TardisTestContainer;
use tardis::web::context_extractor::{TardisContextExtractor, TOKEN_FLAG};
use tardis::web::poem_openapi::{param::Path, payload::Json, Object, OpenApi, Tags};
use tardis::web::web_resp::{TardisApiResult, TardisResp};
use tardis::TardisFuns;

const TLS_KEY: &str = r#"
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAqVYYdfxTT9qr1np22UoIWq4v1E4cHncp35xxu4HNyZsoJBHR
K1gTvwh8x4LMe24lROW/LGWDRAyhaI8qDxxlitm0DPxU8p4iQoDQi3Z+oVKqsSwJ
pd3MRlu+4QFrveExwxgdahXvnhYgFJw5qG/IDWbQM0+ism/yRiXaxFNMI/kXe8FG
+JKSyJzR/yXPqM9ootgIzWxjmV50c+4eyr97DvbwAQcmHi3Ao96p4XoxzKlYWwE9
TA+s0NvmCgYxOdjLEClP8YVKbvSpFMi4dHMZId86xYioeFbr7XPp+2njr9oyZjpd
Xa9Fy5UhwZZqCqh+nQk0m3XUC5pSu3ZrPLxNNQIDAQABAoIBAFKtZJgGsK6md4vq
kyiYSufrcBLaaEQ/rkQtYCJKyC0NAlZKFLRy9oEpJbNLm4cQSkYPXn3Qunx5Jj2k
2MYz+SgIDy7f7KHgr52Ew020dzNQ52JFvBgt6NTZaqL1TKOS1fcJSSNIvouTBerK
NCSXHzfb4P+MfEVe/w1c4ilE+kH9SzdEo2jK/sRbzHIY8TX0JbmQ4SCLLayr22YG
usIxtIYcWt3MMP/G2luRnYzzBCje5MXdpAhlHLi4TB6x4h5PmBKYc57uOVNngKLd
YyrQKcszW4Nx5v0a4HG3A5EtUXNCco1+5asXOg2lYphQYVh2R+1wgu5WiDjDVu+6
EYgjFSkCgYEA0NBk6FDoxE/4L/4iJ4zIhu9BptN8Je/uS5c6wRejNC/VqQyw7SHb
hRFNrXPvq5Y+2bI/DxtdzZLKAMXOMjDjj0XEgfOIn2aveOo3uE7zf1i+njxwQhPu
uSYA9AlBZiKGr2PCYSDPnViHOspVJjxRuAgyWM1Qf+CTC0D95aj0oz8CgYEAz5n4
Cb3/WfUHxMJLljJ7PlVmlQpF5Hk3AOR9+vtqTtdxRjuxW6DH2uAHBDdC3OgppUN4
CFj55kzc2HUuiHtmPtx8mK6G+otT7Lww+nLSFL4PvZ6CYxqcio5MPnoYd+pCxrXY
JFo2W7e4FkBOxb5PF5So5plg+d0z/QiA7aFP1osCgYEAtgi1rwC5qkm8prn4tFm6
hkcVCIXc+IWNS0Bu693bXKdGr7RsmIynff1zpf4ntYGpEMaeymClCY0ppDrMYlzU
RBYiFNdlBvDRj6s/H+FTzHRk2DT/99rAhY9nzVY0OQFoQIXK8jlURGrkmI/CYy66
XqBmo5t4zcHM7kaeEBOWEKkCgYAYnO6VaRtPNQfYwhhoFFAcUc+5t+AVeHGW/4AY
M5qlAlIBu64JaQSI5KqwS0T4H+ZgG6Gti68FKPO+DhaYQ9kZdtam23pRVhd7J8y+
xMI3h1kiaBqZWVxZ6QkNFzizbui/2mtn0/JB6YQ/zxwHwcpqx0tHG8Qtm5ZAV7PB
eLCYhQKBgQDALJxU/6hMTdytEU5CLOBSMby45YD/RrfQrl2gl/vA0etPrto4RkVq
UrkDO/9W4mZORClN3knxEFSTlYi8YOboxdlynpFfhcs82wFChs+Ydp1eEsVHAqtu
T+uzn0sroycBiBfVB949LExnzGDFUkhG0i2c2InarQYLTsIyHCIDEA==
-----END RSA PRIVATE KEY-----
"#;

const TLS_CERT: &str = r#"
-----BEGIN CERTIFICATE-----
MIIEADCCAmigAwIBAgICAcgwDQYJKoZIhvcNAQELBQAwLDEqMCgGA1UEAwwhcG9u
eXRvd24gUlNBIGxldmVsIDIgaW50ZXJtZWRpYXRlMB4XDTE2MDgxMzE2MDcwNFoX
DTIyMDIwMzE2MDcwNFowGTEXMBUGA1UEAwwOdGVzdHNlcnZlci5jb20wggEiMA0G
CSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQCpVhh1/FNP2qvWenbZSghari/UThwe
dynfnHG7gc3JmygkEdErWBO/CHzHgsx7biVE5b8sZYNEDKFojyoPHGWK2bQM/FTy
niJCgNCLdn6hUqqxLAml3cxGW77hAWu94THDGB1qFe+eFiAUnDmob8gNZtAzT6Ky
b/JGJdrEU0wj+Rd7wUb4kpLInNH/Jc+oz2ii2AjNbGOZXnRz7h7Kv3sO9vABByYe
LcCj3qnhejHMqVhbAT1MD6zQ2+YKBjE52MsQKU/xhUpu9KkUyLh0cxkh3zrFiKh4
Vuvtc+n7aeOv2jJmOl1dr0XLlSHBlmoKqH6dCTSbddQLmlK7dms8vE01AgMBAAGj
gb4wgbswDAYDVR0TAQH/BAIwADALBgNVHQ8EBAMCBsAwHQYDVR0OBBYEFMeUzGYV
bXwJNQVbY1+A8YXYZY8pMEIGA1UdIwQ7MDmAFJvEsUi7+D8vp8xcWvnEdVBGkpoW
oR6kHDAaMRgwFgYDVQQDDA9wb255dG93biBSU0EgQ0GCAXswOwYDVR0RBDQwMoIO
dGVzdHNlcnZlci5jb22CFXNlY29uZC50ZXN0c2VydmVyLmNvbYIJbG9jYWxob3N0
MA0GCSqGSIb3DQEBCwUAA4IBgQBsk5ivAaRAcNgjc7LEiWXFkMg703AqDDNx7kB1
RDgLalLvrjOfOp2jsDfST7N1tKLBSQ9bMw9X4Jve+j7XXRUthcwuoYTeeo+Cy0/T
1Q78ctoX74E2nB958zwmtRykGrgE/6JAJDwGcgpY9kBPycGxTlCN926uGxHsDwVs
98cL6ZXptMLTR6T2XP36dAJZuOICSqmCSbFR8knc/gjUO36rXTxhwci8iDbmEVaf
BHpgBXGU5+SQ+QM++v6bHGf4LNQC5NZ4e4xvGax8ioYu/BRsB/T3Lx+RlItz4zdU
XuxCNcm3nhQV2ZHquRdbSdoyIxV5kJXel4wCmOhWIq7A2OBKdu5fQzIAzzLi65EN
RPAKsKB4h7hGgvciZQ7dsMrlGw0DLdJ6UrFyiR5Io7dXYT/+JP91lP5xsl6Lhg9O
FgALt7GSYRm2cZdgi9pO9rRr83Br1VjQT1vHz6yoZMXSqc4A2zcN2a2ZVq//rHvc
FZygs8miAhWPzqnpmgTj1cPiU1M=
-----END CERTIFICATE-----
"#;

#[tokio::test]
async fn test_web_server() -> TardisResult<()> {
    let web_url = "https://localhost:8080";

    let docker = clients::Cli::default();
    let redis_container = TardisTestContainer::redis_custom(&docker);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}/0", redis_port);

    tokio::spawn(async move { start_serv(web_url, &redis_url).await });
    sleep(Duration::from_millis(500)).await;

    test_basic(web_url).await?;
    test_validate(web_url).await?;
    test_context(web_url).await?;
    test_security().await?;

    Ok(())
}

async fn start_serv(web_url: &str, redis_url: &str) -> TardisResult<()> {
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig {
            app: Default::default(),
            web_server: WebServerConfig {
                enabled: true,
                modules: HashMap::from([
                    (
                        "todo".to_string(),
                        WebServerModuleConfig {
                            name: "todo app".to_string(),
                            doc_urls: [("test env".to_string(), web_url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())].to_vec(),
                            ..Default::default()
                        },
                    ),
                    (
                        "other".to_string(),
                        WebServerModuleConfig {
                            name: "other app".to_string(),
                            ..Default::default()
                        },
                    ),
                ]),
                tls_key: Some(TLS_KEY.to_string()),
                tls_cert: Some(TLS_CERT.to_string()),
                ..Default::default()
            },
            web_client: Default::default(),
            cache: CacheConfig {
                enabled: true,
                url: redis_url.to_string(),
                modules: Default::default(),
            },
            db: DBConfig {
                enabled: false,
                ..Default::default()
            },
            mq: MQConfig {
                enabled: false,
                ..Default::default()
            },
            search: SearchConfig {
                enabled: false,
                ..Default::default()
            },
            mail: MailConfig {
                enabled: false,
                ..Default::default()
            },
            os: OSConfig {
                enabled: false,
                ..Default::default()
            },
            adv: Default::default(),
        },
    })
    .await?;
    TardisFuns::web_server().add_module("todo", TodosApi).await.add_module_with_data::<_, String>("other", OtherApi, None).await.start().await
}

async fn test_basic(url: &str) -> TardisResult<()> {
    // Normal
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap().code.to_string(), "code1");

    // Business Error
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1/err", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::Conflict("异常".to_string()).code());
    assert_eq!(response.msg, TardisError::Conflict("异常".to_string()).message());

    // Not Found
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1/ss", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::NotFound("".to_string()).code());
    assert_eq!(response.msg, "not found");

    Ok(())
}

async fn test_validate(url: &str) -> TardisResult<()> {
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/ss", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::NotFound("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"failed to parse path `id`: failed to parse "integer(int64)": invalid digit found in string"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "".to_string(),
                eq: "".to_string(),
                range: 0,
                mail: "".to_string(),
                contain: "".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `len` verification failed. minLength(1)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "".to_string(),
                range: 0,
                mail: "".to_string(),
                contain: "".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `eq` verification failed. minLength(5)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 0,
                mail: "".to_string(),
                contain: "".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `range` verification failed. minimum(1, exclusive: false)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss.ss".to_string(),
                contain: "".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `mail` verification failed. Invalid mail format"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `contain` verification failed. pattern(".*gmail.*")"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
            &ValidateReq {
                len: "1".to_string(),
                eq: "11111".to_string(),
                range: 444,
                mail: "ss@ss.ss".to_string(),
                contain: "gmail".to_string(),
                phone: "".to_string(),
                item_len: vec![],
                item_unique: vec![],
            },
            None,
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `phone` verification failed. Invalid phone number format"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
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
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `item_len` verification failed. minItems(1)"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
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
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(
        response.msg,
        r#"parse request payload error: failed to parse "ValidateReq": field `item_unique` verification failed. uniqueItems()"#
    );

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
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
    let response = TardisFuns::web_client().get::<TardisResp<String>>(format!("{}/other/context_in_header", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::Unauthorized("".to_string()).code());
    assert_eq!(response.msg, "authorization error");

    // from header
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(), "sss".to_string())]),
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::Unauthorized("".to_string()).code());
    assert_eq!(response.msg, "authorization error");

    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(), "c3Nz".to_string())]),
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::Unauthorized("".to_string()).code());
    assert_eq!(response.msg, "authorization error");

    let context = TardisContext {
        own_paths: "tenant1/app1".to_string(),
        ak: "ak1".to_string(),
        roles: vec!["r1".to_string(), "管理员".to_string()],
        groups: vec!["g1".to_string()],
        owner: "acc1".to_string(),
    };
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(
                TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(),
                TardisFuns::json.obj_to_string(&context).unwrap(),
            )]),
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::Unauthorized("".to_string()).code());
    assert_eq!(response.msg, "authorization error");

    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(
                TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(),
                base64::encode(&TardisFuns::json.obj_to_string(&context).unwrap()),
            )]),
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap(), "管理员");

    // from cache
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(
                TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(),
                format!("{}{}", TOKEN_FLAG, "token1").to_string(),
            )]),
        )
        .await?
        .body
        .unwrap();
    assert_eq!(response.code, TardisError::Unauthorized("".to_string()).code());
    assert_eq!(response.msg, "authorization error");

    let context = TardisContext {
        own_paths: "tenant1/app1".to_string(),
        ak: "ak1".to_string(),
        roles: vec!["r1".to_string(), "管理员".to_string()],
        groups: vec!["g1".to_string()],
        owner: "acc1".to_string(),
    };
    TardisFuns::cache()
        .set(
            format!("{}token1", TardisFuns::fw_config().web_server.context_conf.token_cache_key).as_str(),
            TardisFuns::json.obj_to_string(&context).unwrap().as_str(),
        )
        .await
        .unwrap();
    let response = TardisFuns::web_client()
        .get::<TardisResp<String>>(
            format!("{}/other/context_in_header", url).as_str(),
            Some(vec![(
                TardisFuns::fw_config().web_server.context_conf.context_header_name.to_string(),
                format!("{}{}", TOKEN_FLAG, "token1"),
            )]),
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

    tokio::spawn(async {
        TardisFuns::init_conf(TardisConfig {
            cs: Default::default(),
            fw: FrameworkConfig {
                app: Default::default(),
                web_server: WebServerConfig {
                    enabled: true,
                    port: 8081,
                    modules: HashMap::from([
                        (
                            "todo".to_string(),
                            WebServerModuleConfig {
                                name: "todo app".to_string(),
                                doc_urls: [("test env".to_string(), url.to_string()), ("prod env".to_string(), "http://127.0.0.1".to_string())].iter().cloned().collect(),
                                ..Default::default()
                            },
                        ),
                        (
                            "other".to_string(),
                            WebServerModuleConfig {
                                name: "other app".to_string(),
                                ..Default::default()
                            },
                        ),
                    ]),
                    tls_key: Some(TLS_KEY.to_string()),
                    tls_cert: Some(TLS_CERT.to_string()),
                    security_hide_err_msg: true,
                    ..Default::default()
                },
                web_client: Default::default(),
                cache: CacheConfig {
                    enabled: false,
                    ..Default::default()
                },
                db: DBConfig {
                    enabled: false,
                    ..Default::default()
                },
                mq: MQConfig {
                    enabled: false,
                    ..Default::default()
                },
                search: SearchConfig {
                    enabled: false,
                    ..Default::default()
                },
                mail: MailConfig {
                    enabled: false,
                    ..Default::default()
                },
                os: OSConfig {
                    enabled: false,
                    ..Default::default()
                },
                adv: Default::default(),
            },
        })
        .await?;
        TardisFuns::web_server().add_module("todo", TodosApi).await.add_module_with_data::<_, String>("other", OtherApi, None).await.start().await
    });
    sleep(Duration::from_millis(500)).await;

    // Normal
    let response = TardisFuns::web_client()
        .post::<TodoAddReq, TardisResp<String>>(
            format!("{}/todo/todos", url).as_str(),
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

    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TARDIS_RESULT_SUCCESS_CODE);
    assert_eq!(response.data.unwrap().description, "测试");

    // Business Error
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1/err", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::Conflict("异常".to_string()).code());
    assert_eq!(response.msg, "Security is enabled, detailed errors are hidden, please check the server logs");

    // Not Found
    let response = TardisFuns::web_client().get::<TardisResp<TodoResp>>(format!("{}/todo/todos/1/ss", url).as_str(), None).await?.body.unwrap();
    assert_eq!(response.code, TardisError::NotFound("".to_string()).code());
    assert_eq!(response.msg, "Security is enabled, detailed errors are hidden, please check the server logs");

    let response = TardisFuns::web_client()
        .post::<ValidateReq, TardisResp<String>>(
            format!("{}/other/validate", url).as_str(),
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
    assert_eq!(response.code, TardisError::BadRequest("".to_string()).code());
    assert_eq!(response.msg, "Security is enabled, detailed errors are hidden, please check the server logs");

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

#[derive(Object, Serialize, Deserialize, Debug)]
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

    #[oai(path = "/todos/:id/err", method = "get")]
    async fn get_by_error(&self, id: Path<i64>) -> TardisApiResult<TodoResp> {
        TardisResp::err(TardisError::Conflict("异常".to_string()))
    }
}

struct OtherApi;

#[OpenApi]
impl OtherApi {
    #[oai(path = "/validate", method = "post")]
    async fn validate(&self, _req: Json<ValidateReq>) -> TardisApiResult<String> {
        TardisResp::ok("".into())
    }

    #[oai(path = "/context_in_header", method = "get")]
    async fn context_in_header(&self, cxt: TardisContextExtractor) -> TardisApiResult<String> {
        TardisResp::ok(cxt.0.roles.get(1).unwrap().to_string())
    }
}
