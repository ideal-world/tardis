use std::env;

use tardis::basic::result::TardisResult;
use tardis::config::config_dto::{FrameworkConfig, MailModuleConfig, TardisConfig, WebServerConfig};
use tardis::mail::mail_client::TardisMailSendReq;
use tardis::TardisFuns;

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_mail_client() -> TardisResult<()> {
    env::set_var("RUST_LOG", "info,tardis=trace");
    TardisFuns::init_log();    let framework_config = FrameworkConfig::builder()
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
        .send(
            &TardisMailSendReq::builder()
                .subject("测试")
                .txt_body("这是一封测试邮件")
                .html_body("<h1>测试</h1>这是一封测试邮件")
                .to(["<username>@outlook.com".to_string()])
                .build(),
        )
        .await?;
    Ok(())
}
