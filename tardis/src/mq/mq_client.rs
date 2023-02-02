use std::collections::HashMap;
use std::future::Future;

use amq_protocol_types::{AMQPValue, LongString, ShortString};
use futures_util::lock::Mutex;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties, Consumer, ExchangeKind};
use url::Url;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::log::{error, info, trace};

pub struct TardisMQClient {
    con: Connection,
    channels: Mutex<Vec<Channel>>,
}

impl TardisMQClient {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisMQClient>> {
        let mut clients = HashMap::new();
        clients.insert("".to_string(), TardisMQClient::init(&conf.mq.url).await?);
        for (k, v) in &conf.mq.modules {
            clients.insert(k.to_string(), TardisMQClient::init(&v.url).await?);
        }
        Ok(clients)
    }

    pub async fn init(str_url: &str) -> TardisResult<TardisMQClient> {
        let url = Url::parse(str_url).map_err(|_| TardisError::format_error(&format!("[Tardis.MQClient] Invalid url {str_url}"), "406-tardis-mq-url-error"))?;
        info!("[Tardis.MQClient] Initializing, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let con = Connection::connect(str_url, ConnectionProperties::default().with_connection_name("tardis".into())).await?;
        info!("[Tardis.MQClient] Initialized, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        Ok(TardisMQClient {
            con,
            channels: Mutex::new(Vec::new()),
        })
    }

    pub async fn close(&self) -> TardisResult<()> {
        info!("[Tardis.MQClient] Shutdown...");
        let channels = self.channels.lock().await;
        for channel in channels.iter() {
            channel.close(0u16, "Shutdown AMQP Channel").await?;
        }
        self.con.close(0u16, "Shutdown AMQP Connection").await?;
        Ok(())
    }

    pub async fn request(&self, address: &str, message: String, header: &HashMap<String, String>) -> TardisResult<()> {
        trace!("[Tardis.MQClient] Request, queue:{}, message:{}", address, message);
        let channel = self.con.create_channel().await?;
        channel.confirm_select(ConfirmSelectOptions::default()).await?;
        let mut mq_header = FieldTable::default();
        for (k, v) in header {
            mq_header.insert(ShortString::from(k.to_string()), AMQPValue::from(LongString::from(v.to_string())));
        }
        let confirm = channel
            .basic_publish(
                "",
                address,
                BasicPublishOptions::default(),
                message.as_bytes(),
                BasicProperties::default().with_headers(mq_header).with_delivery_mode(2),
            )
            .await?
            .await?;
        if confirm.is_ack() {
            channel.close(200u16, "").await?;
            Ok(())
        } else {
            Err(TardisError::internal_error("MQ request confirmation error", "500-tardis-mq-confirm-error"))
        }
    }

    pub async fn response<F, T>(&self, address: &str, fun: F) -> TardisResult<()>
    where
        F: Fn((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        info!("[Tardis.MQClient] Response, queue:{}", address);
        let channel = self.con.create_channel().await?;
        channel
            .queue_declare(
                address,
                QueueDeclareOptions {
                    passive: false,
                    durable: true,
                    exclusive: false,
                    auto_delete: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await?;
        channel.basic_qos(1, BasicQosOptions::default()).await?;
        let consumer = channel
            .basic_consume(
                address,
                "",
                BasicConsumeOptions {
                    no_local: false,
                    no_ack: false,
                    exclusive: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await?;
        self.channels.lock().await.push(channel);
        self.process(address.to_string(), consumer, fun).await
    }

    pub async fn publish(&self, topic: &str, message: String, header: &HashMap<String, String>) -> TardisResult<()> {
        trace!("[Tardis.MQClient] Publish, queue:{}, message:{}", topic, message);
        let channel = self.con.create_channel().await?;
        channel.confirm_select(ConfirmSelectOptions::default()).await?;
        let mut mq_header = FieldTable::default();
        for (k, v) in header {
            mq_header.insert(ShortString::from(k.to_string()), AMQPValue::from(LongString::from(v.to_string())));
        }
        let confirm = channel
            .basic_publish(
                topic,
                "",
                BasicPublishOptions::default(),
                message.as_bytes(),
                BasicProperties::default().with_headers(mq_header).with_delivery_mode(2),
            )
            .await?
            .await?;
        if confirm.is_ack() {
            channel.close(200u16, "").await?;
            Ok(())
        } else {
            Err(TardisError::internal_error("MQ request confirmation error", "500-tardis-mq-confirm-error"))
        }
    }

    pub async fn subscribe<F, T>(&self, topic: &str, fun: F) -> TardisResult<()>
    where
        F: Fn((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        info!("[Tardis.MQClient] Subscribe, queue:{}", topic);
        let channel = self.con.create_channel().await?;
        self.declare_exchange(&channel, topic).await?;
        let temp_queue_name = channel
            .queue_declare(
                "",
                QueueDeclareOptions {
                    passive: false,
                    durable: false,
                    exclusive: true,
                    auto_delete: true,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await?
            .name()
            .to_string();
        channel.queue_bind(&temp_queue_name, topic, "", QueueBindOptions::default(), FieldTable::default()).await?;
        channel.basic_qos(1, BasicQosOptions::default()).await?;
        let consumer = channel
            .basic_consume(
                &temp_queue_name,
                "",
                BasicConsumeOptions {
                    no_local: false,
                    no_ack: false,
                    exclusive: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await?;
        self.channels.lock().await.push(channel);
        self.process(topic.to_string(), consumer, fun).await
    }

    async fn declare_exchange(&self, channel: &Channel, topic: &str) -> TardisResult<()> {
        channel
            .exchange_declare(
                topic,
                ExchangeKind::Fanout,
                ExchangeDeclareOptions {
                    passive: false,
                    durable: true,
                    auto_delete: false,
                    internal: false,
                    nowait: false,
                },
                FieldTable::default(),
            )
            .await?;
        Ok(())
    }

    async fn process<F, T>(&self, topic_or_address: String, mut consumer: Consumer, fun: F) -> TardisResult<()>
    where
        F: Fn((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
        T: Future<Output = TardisResult<()>> + Send + 'static,
    {
        async_global_executor::spawn(async move {
            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(d) => match std::str::from_utf8(d.data.as_slice()) {
                        Ok(msg) => {
                            trace!("[Tardis.MQClient] Receive, queue:{}, message:{}", topic_or_address, msg);
                            let mut resp_header: HashMap<String, String> = HashMap::default();
                            let _ = d.properties.headers().as_ref().map(|header| {
                                for (k, v) in header.into_iter() {
                                    let value = if let AMQPValue::LongString(val) = v {
                                        val.to_string()
                                    } else {
                                        error!("[Tardis.MQClient] Receive, queue:{topic_or_address}, message:{msg} | MQ Header only supports string types");
                                        panic!("[Tardis.MQClient] Receive, queue:{topic_or_address}, message:{msg} | MQ Header only supports string types")
                                    };
                                    resp_header.insert(k.to_string(), value);
                                }
                            });
                            match fun((resp_header, msg.to_string())).await {
                                Ok(_) => match d.ack(BasicAckOptions::default()).await {
                                    Ok(_) => (),
                                    Err(error) => {
                                        error!("[Tardis.MQClient] Receive ack error, queue:{topic_or_address}, message:{msg} | {error}");
                                    }
                                },
                                Err(error) => {
                                    error!("[Tardis.MQClient] Receive process error, queue:{topic_or_address}, message:{msg} | {error}");
                                }
                            }
                        }
                        Err(error) => {
                            error!("[Tardis.MQClient] Receive delivery error, queue:{topic_or_address} | {error}");
                        }
                    },
                    Err(error) => {
                        error!("[Tardis.MQClient] Receive connection error, queue:{topic_or_address} | {error}");
                    }
                }
            }
        })
        .detach();
        Ok(())
    }
}

impl From<lapin::Error> for TardisError {
    fn from(error: lapin::Error) -> Self {
        error!("[Tardis.MQClient] Error: {}", error.to_string());
        TardisError::wrap(&format!("[Tardis.MQClient] {error:?}"), "-1-tardis-mq-error")
    }
}
