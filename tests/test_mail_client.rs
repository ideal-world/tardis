use tardis::basic::config::{CacheConfig, DBConfig, FrameworkConfig, MQConfig, MailConfig, OSConfig, SearchConfig, TardisConfig, WebServerConfig};
use tardis::basic::result::TardisResult;
use tardis::mail::mail_client::TardisMailSendReq;
use tardis::TardisFuns;

#[tokio::test]
#[ignore]
async fn test_mail_client() -> TardisResult<()> {
    TardisFuns::init_log()?;
    TardisFuns::init_conf(TardisConfig {
        cs: Default::default(),
        fw: FrameworkConfig {
            app: Default::default(),
            web_server: WebServerConfig {
                enabled: false,
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
                enabled: true,
                smtp_host: "smtp.163.com".to_string(),
                smtp_port: 465,
                smtp_username: "<username>".to_string(),
                smtp_password: "<password>".to_string(),
                default_from: "<username>@163.com".to_string(),
                modules: Default::default(),
            },
            os: OSConfig {
                enabled: false,
                ..Default::default()
            },
            adv: Default::default(),
        },
    })
    .await?;

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
