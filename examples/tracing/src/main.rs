use std::collections::HashMap;

use processor::TaskKind;
use tardis::basic::result::TardisResult;
use tardis::rand::random;
use tardis::tokio;
use tracing::Instrument;
use tracing_subscriber::fmt::format::FmtSpan;

mod processor;

#[tokio::main]
async fn main() -> TardisResult<()> {
    let mut threads = Vec::new();
    tracing_subscriber::fmt().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE).with_max_level(tracing::Level::DEBUG).with_thread_ids(true).try_init().unwrap();

    for idx in 0..10 {
        let thread = tokio::spawn(async move {
            let kind = TaskKind::try_from((random::<u32>() % 4) + 1).unwrap();
            let params = gen_params(&kind);
            processor::dispatch(kind, params).await.unwrap();
        })
        .instrument(tracing::info_span!("task", idx));
        threads.push(thread);
    }

    for thread in threads.into_iter() {
        thread.await.unwrap();
    }
    Ok(())
}

fn gen_params(kind: &TaskKind) -> HashMap<String, String> {
    let mut params = HashMap::new();
    match kind {
        TaskKind::SendEmail => {
            params.insert("user_id".to_string(), mock_user_id());
            params.insert("email".to_string(), mock_email());
            params.insert("content".to_string(), mock_content());
        }
        TaskKind::SendSms => {
            params.insert("user_id".to_string(), mock_user_id());
            params.insert("phone".to_string(), mock_phone_num());
            params.insert("content".to_string(), mock_content());
        }
        TaskKind::SendPush => {
            params.insert("user_id".to_string(), mock_user_id());
            params.insert("token".to_string(), mock_token());
            params.insert("content".to_string(), mock_content());
        }
        TaskKind::ExportExcel => {
            params.insert("import_path".to_string(), mock_data_path());
        }
        TaskKind::GenerateImage => {
            params.insert("export_url".to_string(), mock_img_url());
        }
    }
    params
}

fn mock_phone_num() -> String {
    let head_charset = "3456789";
    let tail_charset = "0123456789";
    format!("1{}{}", random_string::generate(2, head_charset), random_string::generate(8, tail_charset))
}

fn mock_email() -> String {
    let charset = "0123456789qwertyyuiopasdfghjklzxcvbnm";
    format!("{}@gmail.com", random_string::generate(8, charset))
}

fn mock_user_id() -> String {
    let charset = "123456789";
    random_string::generate(6, charset)
}

fn mock_token() -> String {
    let charset = "0123456789qwertyyuiopasdfghjklzxcvbnm";
    random_string::generate(16, charset)
}

fn mock_content() -> String {
    "test".to_string()
}

fn mock_data_path() -> String {
    let charset = "0123456789qwertyyuiopasdfghjklzxcvbnm";
    format!("./data/export/tmp/{}.csv", random_string::generate(12, charset))
}

fn mock_img_url() -> String {
    let charset = "0123456789qwertyyuiopasdfghjklzxcvbnm";
    format!("www.xxx.com/oss/{}.jpg", random_string::generate(13, charset))
}
