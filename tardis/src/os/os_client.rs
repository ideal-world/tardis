use std::collections::HashMap;
use std::ops::Deref;

use async_trait::async_trait;
use log::{error, info, trace};
use s3::creds::Credentials;
use s3::{Bucket, BucketConfiguration, Region};

use crate::basic::error::{TardisError, ERROR_DEFAULT_CODE};
use crate::{FrameworkConfig, TardisResult};

pub struct TardisOSClient {
    client: Box<dyn TardisOSOperations + Sync + Send>,
}

struct TardisOSS3Client {
    region: Region,
    credentials: Credentials,
    default_bucket: Option<Bucket>,
}

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
                    expiration: None,
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
            _ => Err(TardisError::not_implemented(
                &format!("[Tardis.OSClient] Unsupported OS kind {kind}"),
                "501-tardis-os-kind-error",
            )),
        }
    }

    fn get_client(&self) -> &(dyn TardisOSOperations + Sync + Send) {
        self.client.deref()
    }

    pub async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Creating bucket {}", bucket_name);
        self.get_client().bucket_create_simple(bucket_name, is_private).await
    }

    pub async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Deleting bucket {}", bucket_name);
        self.get_client().bucket_delete(bucket_name).await
    }

    pub async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<String>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Creating object {}", path);
        self.get_client().object_create(path, content, content_type, bucket_name).await
    }

    pub async fn object_get(&self, path: &str, bucket_name: Option<String>) -> TardisResult<Vec<u8>> {
        trace!("[Tardis.OSClient] Getting object {}", path);
        self.get_client().object_get(path, bucket_name).await
    }

    pub async fn object_delete(&self, path: &str, bucket_name: Option<String>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Deleting object {}", path);
        self.get_client().object_delete(path, bucket_name).await
    }

    pub fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Creating object url {}", path);
        self.get_client().object_create_url(path, expire_sec, bucket_name)
    }

    pub fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Getting object url {}", path);
        self.get_client().object_get_url(path, expire_sec, bucket_name)
    }

    pub fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Deleting object url {}", path);
        self.get_client().object_delete_url(path, expire_sec, bucket_name)
    }
}

#[async_trait]
trait TardisOSOperations {
    async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()>;

    async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()>;

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<String>) -> TardisResult<()>;

    async fn object_get(&self, path: &str, bucket_name: Option<String>) -> TardisResult<Vec<u8>>;

    async fn object_delete(&self, path: &str, bucket_name: Option<String>) -> TardisResult<()>;

    fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String>;

    fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String>;

    fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String>;
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
            Err(TardisError::custom(
                &resp.response_code.to_string(),
                &format!(
                    "[Tardis.OSClient] Failed to create bucket {} with error [{}]{}",
                    bucket_name, resp.response_code, resp.response_text
                ),
                "-1-tardis-os-create-bucket-error",
            ))
        }
    }

    async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()> {
        let code = Bucket::new(bucket_name, self.region.clone(), self.credentials.clone())?.with_path_style().delete().await?;
        if code == 200 || code == 204 {
            Ok(())
        } else {
            Err(TardisError::custom(
                &code.to_string(),
                &format!("[Tardis.OSClient] Failed to delete bucket {bucket_name} with error [{code}]"),
                "-1-tardis-os-delete-bucket-error",
            ))
        }
    }

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<String>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let response_data = if let Some(content_type) = content_type {
            bucket.put_object_with_content_type(path, content, content_type).await?
        } else {
            bucket.put_object(path, content).await?
        };
        if response_data.status_code() == 200 {
            Ok(())
        } else {
            Err(TardisError::custom(
                &response_data.status_code().to_string(),
                &format!(
                    "[Tardis.OSClient] Failed to create object {}:{} with error [{}]",
                    bucket.name,
                    path,
                    std::str::from_utf8(response_data.bytes())?
                ),
                "-1-tardis-os-create-object-error",
            ))
        }
    }

    async fn object_get(&self, path: &str, bucket_name: Option<String>) -> TardisResult<Vec<u8>> {
        let bucket = self.get_bucket(bucket_name)?;
        let response_data = bucket.get_object(path).await?;
        if response_data.status_code() == 200 {
            Ok(response_data.bytes().to_vec())
        } else {
            Err(TardisError::custom(
                &response_data.status_code().to_string(),
                &format!(
                    "[Tardis.OSClient] Failed to get object {}:{} with error [{}]",
                    bucket.name,
                    path,
                    std::str::from_utf8(response_data.bytes())?
                ),
                "-1-tardis-os-get-object-error",
            ))
        }
    }

    async fn object_delete(&self, path: &str, bucket_name: Option<String>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let response_data = bucket.delete_object(path).await?;
        if response_data.status_code() == 200 || response_data.status_code() == 204 {
            Ok(())
        } else {
            Err(TardisError::custom(
                &response_data.status_code().to_string(),
                &format!(
                    "[Tardis.OSClient] Failed to delete object {}:{} with error [{}]",
                    bucket.name,
                    path,
                    std::str::from_utf8(response_data.bytes())?
                ),
                "-1-tardis-os-delete-object-error",
            ))
        }
    }

    fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_put(path, expire_sec, None)?)
    }

    fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_get(path, expire_sec, None)?)
    }

    fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<String>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_delete(path, expire_sec)?)
    }
}

impl TardisOSS3Client {
    fn get_bucket(&self, bucket_name: Option<String>) -> TardisResult<Bucket> {
        if let Some(bucket_name) = bucket_name {
            Ok(Bucket::new(&bucket_name, self.region.clone(), self.credentials.clone())?.with_path_style())
        } else {
            let bucket =
                self.default_bucket.as_ref().ok_or_else(|| TardisError::not_found("[Tardis.OSClient] No default bucket configured", "404-tardis-os-default-bucket-not-exist"))?;
            Ok(bucket.clone())
        }
    }
}

impl From<s3::error::S3Error> for TardisError {
    fn from(error: s3::error::S3Error) -> Self {
        error!("[Tardis.OSClient] Error: {}", error.to_string());
        match error {
            s3::error::S3Error::Http(http_code, msg) => TardisError::custom(&format!("{http_code}"), &format!("[Tardis.OSClient] Error: {msg}"), "-1-tardis-os-error"),
            _ => TardisError::custom(ERROR_DEFAULT_CODE, &format!("[Tardis.OSClient] Error: {error:?}"), "-1-tardis-os-error"),
        }
    }
}
