//!
//!
//!
//!
//! Protocol:
//!
//! 1. Route
//! 2. Avatar
//! 3. Forward

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap};

use crate::{
    basic::result::TardisResult,
    cluster::cluster_processor::{TardisClusterMessageReq, TardisClusterSubscriber},
};

use super::ws_insts_mapping_avatars;

pub const EVENT_AVATAR: &str = "tardis/avatar";

pub(crate) struct Avatar;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum AvatarMessage {
    Sync { table: HashMap<String, Vec<String>> },

}

#[async_trait::async_trait]
impl TardisClusterSubscriber for Avatar {
    fn event_name(&self) -> Cow<'static, str> {
        EVENT_AVATAR.into()
    }

    async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
        // let from_node = message_req.req_node_id;
        if let Ok(message) = serde_json::from_value(message_req.msg) {
            match message {
                AvatarMessage::Sync { table } => {
                    let mut routes = ws_insts_mapping_avatars().write().await;
                    for (k, v) in table {
                        routes.insert(k, v);
                    }
                }
            }
        }
        Ok(None)
    }
}

// pub(crate) struct Forward {}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub(crate) enum ForwardMessage {
//     Forward {
//         to_inst: String,
//         message: String,
//     },
// }

// #[async_trait::async_trait]
// impl TardisClusterSubscriber for Forward {
//     fn event_name(&self) -> Cow<'static, str> {
//         "cluster/forward".into()
//     }

//     async fn subscribe(&self, message_req: TardisClusterMessageReq) -> TardisResult<Option<Value>> {
//         // let from_node = message_req.req_node_id;
//         if let Ok(message) = serde_json::from_value(message_req.msg) {
//             match message {
//                 ForwardMessage::Forward { to_inst, message } => {

//                 }
//             }
//         }
//         Ok(None)
//     }
// }