use std::env;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, MailModuleConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig};
use tardis::mail::mail_client::TardisMailSendReq;
use tardis::TardisFuns;

#[tokio::test]
#[ignore]
async fn test_mail_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log()?;
    let framework_config = FrameworkConfig::builder()
        .web_server(WebServerConfig::default())
        .mail(
            MailModuleConfig::builder()
                .smtp_host("smtp.163.com")
                .smtp_port(465)
                .smtp_username("<username>")
                .smtp_password("<password>")
                .default_from("<username>@163.com")
                .starttls(false)
                .build(),
        )
        .build();
    TardisFuns::init_conf(TardisConfig::builder().fw(framework_config).build()).await?;

    TardisFuns::mail()
        .send(&TardisMailSendReq {
            subject: "测试".to_string(),
            txt_body: "这是一封测试邮件".to_string(),
            html_body: Some("<h1>测试</h1>这是一封测试邮件".to_string()),
            to: vec!["<username>@outlook.com".to_string()],
            reply_to: None,
            cc: None,
            bcc: None,
            from: None,
        })
        .await?;

    Ok(())
}
