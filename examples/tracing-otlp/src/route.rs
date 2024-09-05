use std::collections::HashMap;

use tardis::log::instrument;
use tardis::web::poem_openapi;
use tardis::web::web_resp::{TardisApiResult, TardisResp};

use crate::processor::{self, TaskKind};

#[derive(Debug, Clone)]
pub struct Api;

#[poem_openapi::OpenApi]
impl Api {
    #[oai(path = "/send_email", method = "get")]
    async fn send_email(&self) -> TardisApiResult<String> {
        let params = gen_params(&TaskKind::SendEmail);
        processor::dispatch(TaskKind::SendEmail, params).await.unwrap();
        TardisResp::ok("send email".to_string())
    }

    #[instrument(level = "debug")]
    #[oai(path = "/send_sms", method = "get")]
    async fn send_sms(&self) -> TardisApiResult<String> {
        let params = gen_params(&TaskKind::SendSms);
        processor::dispatch(TaskKind::SendSms, params).await.unwrap();
        TardisResp::ok("send sms".to_string())
    }

    #[instrument(level = "debug")]
    #[oai(path = "/send_push", method = "get")]
    async fn send_push(&self) -> TardisApiResult<String> {
        let params = gen_params(&TaskKind::SendPush);
        processor::dispatch(TaskKind::SendPush, params).await.unwrap();
        TardisResp::ok("send push".to_string())
    }

    #[instrument(level = "debug")]
    #[oai(path = "/export_excel", method = "get")]
    async fn export_excel(&self) -> TardisApiResult<String> {
        let params = gen_params(&TaskKind::ExportExcel);
        processor::dispatch(TaskKind::ExportExcel, params).await.unwrap();
        TardisResp::ok("send push".to_string())
    }

    #[instrument(level = "debug")]
    #[oai(path = "/generate_image", method = "get")]
    async fn generate_image(&self) -> TardisApiResult<String> {
        let params = gen_params(&TaskKind::GenerateImage);
        processor::dispatch(TaskKind::GenerateImage, params).await.unwrap();
        TardisResp::ok("send push".to_string())
    }
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
