use std::collections::HashMap;
use std::ops::Deref;

use async_trait::async_trait;
use log::info;
use s3::creds::Credentials;
use s3::{Bucket, BucketConfiguration, Region};

use crate::basic::error::TardisError;
use crate::{FrameworkConfig, TardisResult};

pub struct TardisOSClient {
    client: Box<dyn TardisOSOperations + Sync + Send>,
}

struct TardisOSS3Client {
    region: Region,
    credentials: Credentials,
    default_bucket: Option<Bucket>,
}

// struct TardisOSOSSClient {}
//
// struct TardisOSOBSClient {}

impl TardisOSClient {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisOSClient>> {
        let mut clients = HashMap::new();
        clients.insert(
            "".to_string(),
            TardisOSClient::init(&conf.os.kind, &conf.os.endpoint, &conf.os.ak, &conf.os.sk, &conf.os.region, &conf.os.default_bucket)?,
        );
        for (k, v) in &conf.os.modules {
            clients.insert(k.to_string(), TardisOSClient::init(&v.kind, &v.endpoint, &v.ak, &v.sk, &v.region, &v.default_bucket)?);
        }
        Ok(clients)
    }

    pub fn init(kind: &str, endpoint: &str, ak: &str, sk: &str, region: &str, default_bucket: &str) -> TardisResult<TardisOSClient> {
        info!("[Tardis.OSClient] Initializing for {}", kind);
        match kind {
            "s3" => {
                let region = Region::Custom {
                    region: region.to_string(),
                    endpoint: endpoint.to_string(),
                };
                let credentials = Credentials {
                    access_key: Some(ak.to_string()),
                    secret_key: Some(sk.to_string()),
                    security_token: None,
                    session_token: None,
                };
                let default_bucket = if !default_bucket.is_empty() {
                    Some(Bucket::new(default_bucket, region.clone(), credentials.clone())?.with_path_style())
                } else {
                    None
                };
                let s3 = TardisOSS3Client {
                    region,
                    credentials,
                    default_bucket,
                };
                info!("[Tardis.OSClient] Initialized");
                Ok(TardisOSClient { client: Box::new(s3) })
            }
            _ => Err(TardisError::BadRequest(format!("[Tardis.OSClient] Unsupported OS kind {}", kind))),
        }
    }

    fn get_client(&self) -> &(dyn TardisOSOperations + Sync + Send) {
        self.client.deref()
    }

    pub async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()> {
        self.get_client().bucket_create_simple(bucket_name, is_private).await
    }

    pub async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
        self.get_client().bucket_delete(bucket_name).await
    }

    pub async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
        self.get_client().object_create(path, content, content_type, bucket_name).await
    }

    pub async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
        self.get_client().object_get(path, bucket_name).await
    }

    pub async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
        self.get_client().object_delete(path, bucket_name).await
    }

    pub fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        self.get_client().object_create_url(path, expire_sec, bucket_name)
    }

    pub fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        self.get_client().object_get_url(path, expire_sec, bucket_name)
    }

    pub fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        self.get_client().object_delete_url(path, expire_sec, bucket_name)
    }
}

#[async_trait]
trait TardisOSOperations {
    async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()>;

    async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()>;

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()>;

    async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>>;

    async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()>;

    fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;

    fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;

    fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;
}

#[async_trait]
impl TardisOSOperations for TardisOSS3Client {
    async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()> {
        let resp = Bucket::create_with_path_style(
            bucket_name,
            self.region.clone(),
            self.credentials.clone(),
            if is_private { BucketConfiguration::private() } else { BucketConfiguration::public() },
        )
        .await?;
        if resp.success() {
            Ok(())
        } else {
            Err(TardisError::_Inner(format!(
                "[Tardis.OSClient] Failed to create bucket {} with error [{}]{}",
                bucket_name, resp.response_code, resp.response_text
            )))
        }
    }

    async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
        let code = Bucket::new(bucket_name, self.region.clone(), self.credentials.clone())?.with_path_style().delete().await?;
        if code == 200 || code == 204 {
            Ok(())
        } else {
            Err(TardisError::_Inner(format!(
                "[Tardis.OSClient] Failed to delete bucket {} with error [{}]",
                bucket_name, code
            )))
        }
    }

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let (_, code) = if let Some(content_type) = content_type {
            bucket.put_object_with_content_type(path, content, content_type).await?
        } else {
            bucket.put_object(path, content).await?
        };
        if code == 200 {
            Ok(())
        } else {
            Err(TardisError::_Inner(format!(
                "[Tardis.OSClient] Failed to create object {}:{} with error [{}]",
                bucket.name, path, code
            )))
        }
    }

    async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
        let bucket = self.get_bucket(bucket_name)?;
        let (data, code) = bucket.get_object(path).await?;
        if code == 200 {
            Ok(data)
        } else {
            Err(TardisError::_Inner(format!(
                "[Tardis.OSClient] Failed to get object {}:{} with error [{}]",
                bucket.name, path, code
            )))
        }
    }

    async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let (_, code) = bucket.delete_object(path).await?;
        if code == 200 || code == 204 {
            Ok(())
        } else {
            Err(TardisError::_Inner(format!(
                "[Tardis.OSClient] Failed to delete object {}:{} with error [{}]",
                bucket.name, path, code
            )))
        }
    }

    fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_put(path, expire_sec, None)?)
    }

    fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_get(path, expire_sec, None)?)
    }

    fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_delete(path, expire_sec)?)
    }
}

impl TardisOSS3Client {
    fn get_bucket(&self, bucket_name: Option<&str>) -> TardisResult<Bucket> {
        if let Some(bucket_name) = bucket_name {
            Ok(Bucket::new(bucket_name, self.region.clone(), self.credentials.clone())?.with_path_style())
        } else {
            let bucket = self.default_bucket.as_ref().ok_or_else(|| TardisError::BadRequest("[Tardis.OSClient] No default bucket configured".to_string()))?;
            Ok(bucket.clone())
        }
    }
}

// #[async_trait]
// impl TardisOSOperations for TardisOSOSSClient {
//     async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
//         // TODO
//         Ok(vec![])
//     }
//
//     async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
//
//     fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
//
//     fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
// }
//
// #[async_trait]
// impl TardisOSOperations for TardisOSOBSClient {
//     async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
//         // TODO
//         Ok(vec![])
//     }
//
//     async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
//         // TODO
//         Ok(())
//     }
//
//     fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
//
//     fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
//
//     fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
//         // TODO
//         Ok("".to_string())
//     }
// }

impl From<s3::error::S3Error> for TardisError {
    fn from(error: s3::error::S3Error) -> Self {
        TardisError::_Inner(error.to_string())
    }
}
