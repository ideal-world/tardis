use std::collections::HashMap;
use std::future::Future;

use amq_protocol_types::{AMQPValue, LongString, ShortString};
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties, Consumer, ExchangeKind};
use log::{error, info, trace};
use url::Url;

use crate::basic::config::FrameworkConfig;
use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;

// TODO Elegant closure

pub struct TardisMQClient {
    con: Connection,
}

impl TardisMQClient {
    pub async fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisMQClient> {
        TardisMQClient::init(&conf.mq.url).await
    }

    pub async fn init(str_url: &str) -> TardisResult<TardisMQClient> {
        let url = Url::parse(str_url)?;
        info!("[Tardis.MQClient] Initializing, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        let con = Connection::connect(str_url, ConnectionProperties::default().with_connection_name("tardis".into())).await?;
        info!("[Tardis.MQClient] Initialized, host:{}, port:{}", url.host_str().unwrap_or(""), url.port().unwrap_or(0));
        Ok(TardisMQClient { con })
    }

    pub async fn request(&mut self, address: &str, message: String, header: &HashMap<String, String>) -> TardisResult<()> {
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
            Ok(())
        } else {
            Err(TardisError::InternalError("MQ request confirmation error".to_string()))
        }
    }

    pub async fn response<F, T>(&mut self, address: &str, fun: F) -> TardisResult<()>
    where
        F: FnMut((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
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
        self.process(address.to_string(), consumer, fun).await
    }

    pub async fn publish(&mut self, topic: &str, message: String, header: &HashMap<String, String>) -> TardisResult<()> {
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
            Ok(())
        } else {
            Err(TardisError::InternalError("MQ request confirmation error".to_string()))
        }
    }

    pub async fn subscribe<F, T>(&mut self, topic: &str, fun: F) -> TardisResult<()>
    where
        F: FnMut((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
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
        self.process(topic.to_string(), consumer, fun).await
    }

    async fn declare_exchange(&mut self, channel: &Channel, topic: &str) -> TardisResult<()> {
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

    async fn process<F, T>(&mut self, topic_or_address: String, mut consumer: Consumer, mut fun: F) -> TardisResult<()>
    where
        F: FnMut((HashMap<String, String>, String)) -> T + Send + Sync + 'static,
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
                                        error!(
                                            "[Tardis.MQClient] Receive, queue:{}, message:{} | MQ Header only supports string types",
                                            topic_or_address, msg
                                        );
                                        panic!(
                                            "[Tardis.MQClient] Receive, queue:{}, message:{} | MQ Header only supports string types",
                                            topic_or_address, msg
                                        )
                                    };
                                    resp_header.insert(k.to_string(), value);
                                }
                            });
                            match fun((resp_header, msg.to_string())).await {
                                Ok(_) => match d.ack(BasicAckOptions::default()).await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        error!("[Tardis.MQClient] Receive, queue:{}, message:{} | {}", topic_or_address, msg, e.to_string());
                                    }
                                },
                                Err(e) => {
                                    error!("[Tardis.MQClient] Receive, queue:{}, message:{} | {}", topic_or_address, msg, e.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            error!("[Tardis.MQClient] Receive, queue:{} | {}", topic_or_address, e.to_string());
                        }
                    },
                    Err(e) => {
                        error!("[Tardis.MQClient] Receive, queue:{} | {}", topic_or_address, e.to_string());
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
        TardisError::Box(Box::new(error))
    }
}
