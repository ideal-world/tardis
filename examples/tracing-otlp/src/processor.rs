use std::collections::HashMap;
use tardis::async_trait::async_trait;
use tardis::basic::{error::TardisError, result::TardisResult};
use tardis::tokio;
use tardis::tokio::time::{sleep, Duration};
use tardis::log::{debug_span, info, info_span, instrument};
use tracing::Instrument;

#[derive(Debug)]
pub enum TaskKind {
    SendEmail,
    SendSms,
    SendPush,
    ExportExcel,
    GenerateImage,
}

impl TryFrom<u32> for TaskKind {
    type Error = TardisError;
    fn try_from(value: u32) -> TardisResult<Self> {
        match value {
            1 => Ok(TaskKind::SendEmail),
            2 => Ok(TaskKind::SendSms),
            3 => Ok(TaskKind::SendPush),
            4 => Ok(TaskKind::ExportExcel),
            5 => Ok(TaskKind::GenerateImage),
            _ => Err(TardisError::not_implemented("[Tardis.Task] Unsupported Task kind", "501-tardis-os-kind-error")),
        }
    }
}

#[async_trait]
trait Task {
    async fn handle(&self, params: HashMap<String, String>) -> TardisResult<()>;
}

#[derive(Debug)]
struct SendEmailTask;

#[async_trait]
impl Task for SendEmailTask {
    #[instrument]
    async fn handle(&self, _params: HashMap<String, String>) -> TardisResult<()> {
        sleep(Duration::from_millis(300)).await;
        tardis::TardisFuns::web_client().get_to_str("http://localhost:8089/send_sms", None).await?;
        tokio::spawn(
            async move {
                let span = debug_span!("into spawn");
                let _enter = span.enter();

                log_user().await.unwrap();
            }
            .instrument(tracing::info_span!("task")),
        );
        Ok(())
    }
}

#[derive(Debug)]
struct SendSmsTask;

#[async_trait]
impl Task for SendSmsTask {
    #[instrument]
    async fn handle(&self, _params: HashMap<String, String>) -> TardisResult<()> {
        info!("send sms task is handled");
        sleep(Duration::from_millis(300)).await;
        log_user().await.unwrap();
        Ok(())
    }
}

#[derive(Debug)]
struct SendPushTask;

#[async_trait]
impl Task for SendPushTask {
    #[instrument]
    async fn handle(&self, _params: HashMap<String, String>) -> TardisResult<()> {
        sleep(Duration::from_millis(300)).await;
        notify_admin().await.unwrap();
        Ok(())
    }
}

#[derive(Debug)]
struct ExportExcelTask;

#[async_trait]
impl Task for ExportExcelTask {
    #[instrument]
    async fn handle(&self, _params: HashMap<String, String>) -> TardisResult<()> {
        sleep(Duration::from_millis(300)).await;
        notify_admin().await.unwrap();
        Ok(())
    }
}

#[derive(Debug)]
struct GenerateImageTask;

#[async_trait]
impl Task for GenerateImageTask {
    #[instrument]
    async fn handle(&self, _params: HashMap<String, String>) -> TardisResult<()> {
        sleep(Duration::from_millis(500)).await;
        notify_admin().await.unwrap();
        Ok(())
    }
}

pub async fn dispatch(task_kind: TaskKind, params: HashMap<String, String>) -> TardisResult<()> {
    info!("task dispatch");
    match task_kind {
        TaskKind::SendEmail => SendEmailTask.handle(params).await,
        TaskKind::SendSms => SendSmsTask.handle(params).await,
        TaskKind::SendPush => SendPushTask.handle(params).await,
        TaskKind::ExportExcel => ExportExcelTask.handle(params).await,
        TaskKind::GenerateImage => GenerateImageTask.handle(params).await,
    }
}

#[instrument]
async fn notify_admin() -> TardisResult<()> {
    sleep(Duration::from_millis(100)).await;
    Ok(())
}

#[instrument]
async fn log_user() -> TardisResult<()> {
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
