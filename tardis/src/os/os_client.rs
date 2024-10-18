use std::collections::HashMap;
use std::ops::Deref;

use async_trait::async_trait;
use s3::creds::Credentials;
use s3::serde_types::{BucketLifecycleConfiguration, Part};
use s3::{Bucket, BucketConfiguration, Region};
use tracing::{error, info, trace};

use crate::basic::error::{TardisError, ERROR_DEFAULT_CODE};
use crate::config::config_dto::component::os::OSModuleConfig;
use crate::utils::initializer::InitBy;
use crate::TardisResult;

pub struct TardisOSClient {
    client: Box<dyn TardisOSOperations + Sync + Send>,
}

struct TardisOSS3Client {
    region: Region,
    credentials: Credentials,
    default_bucket: Option<Box<Bucket>>,
}

#[async_trait::async_trait]
impl InitBy<OSModuleConfig> for TardisOSClient {
    async fn init_by(config: &OSModuleConfig) -> TardisResult<Self> {
        Self::init(config)
    }
}

impl TardisOSClient {
    pub fn init(
        OSModuleConfig {
            kind,
            endpoint,
            ak,
            sk,
            region,
            default_bucket,
        }: &OSModuleConfig,
    ) -> TardisResult<TardisOSClient> {
        info!("[Tardis.OSClient] Initializing for {}", kind);
        match kind.as_str() {
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

    pub async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Creating object {}", path);
        self.get_client().object_create(path, content, content_type, bucket_name).await
    }

    pub async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
        trace!("[Tardis.OSClient] Getting object {}", path);
        self.get_client().object_get(path, bucket_name).await
    }

    pub async fn object_exist(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<bool> {
        trace!("[Tardis.OSClient] Head object {}", path);
        self.get_client().object_exist(path, bucket_name).await
    }

    pub async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Deleting object {}", path);
        self.get_client().object_delete(path, bucket_name).await
    }

    pub async fn object_copy(&self, from: &str, to: &str, bucket_name: Option<&str>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Copy object from {} to {}", from, to);
        self.get_client().object_copy(from, to, bucket_name).await
    }

    pub async fn initiate_multipart_upload(&self, path: &str, content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Initiate multipart upload {}", path);
        self.get_client().initiate_multipart_upload(path, content_type, bucket_name).await
    }

    pub async fn batch_build_create_presign_url(&self, path: &str, upload_id: &str, part_number: u32, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<Vec<String>> {
        trace!("[Tardis.OSClient] Batch build create presign url {}", path);
        self.get_client().batch_build_create_presign_url(path, upload_id, part_number, expire_sec, bucket_name).await
    }

    pub async fn complete_multipart_upload(&self, path: &str, upload_id: &str, parts: Vec<String>, bucket_name: Option<&str>) -> TardisResult<()> {
        trace!("[Tardis.OSClient] Complete multipart upload {}", path);
        self.get_client().complete_multipart_upload(path, upload_id, parts, bucket_name).await
    }

    pub async fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Creating object url {}", path);
        self.get_client().object_create_url(path, expire_sec, bucket_name).await
    }

    pub async fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Getting object url {}", path);
        self.get_client().object_get_url(path, expire_sec, bucket_name).await
    }

    pub async fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        trace!("[Tardis.OSClient] Deleting object url {}", path);
        self.get_client().object_delete_url(path, expire_sec, bucket_name).await
    }

    pub async fn get_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<BucketLifecycleConfiguration> {
        self.get_client().get_lifecycle(bucket_name).await
    }

    pub async fn put_lifecycle(&self, bucket_name: Option<&str>, config: BucketLifecycleConfiguration) -> TardisResult<()> {
        self.get_client().put_lifecycle(bucket_name, config).await
    }

    pub async fn delete_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<()> {
        self.get_client().delete_lifecycle(bucket_name).await
    }
}

#[async_trait]
trait TardisOSOperations {
    async fn bucket_create_simple(&self, bucket_name: &str, is_private: bool) -> TardisResult<()>;

    async fn bucket_delete(&self, bucket_name: &str) -> TardisResult<()>;

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()>;

    async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>>;

    async fn object_exist(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<bool>;

    async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()>;

    async fn object_copy(&self, from: &str, to: &str, bucket_name: Option<&str>) -> TardisResult<()>;

    async fn initiate_multipart_upload(&self, path: &str, content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<String>;

    async fn batch_build_create_presign_url(&self, path: &str, upload_id: &str, part_number: u32, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<Vec<String>>;

    async fn complete_multipart_upload(&self, path: &str, upload_id: &str, parts: Vec<String>, bucket_name: Option<&str>) -> TardisResult<()>;

    async fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;

    async fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;

    async fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String>;

    async fn get_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<BucketLifecycleConfiguration>;

    async fn put_lifecycle(&self, bucket_name: Option<&str>, config: BucketLifecycleConfiguration) -> TardisResult<()>;

    async fn delete_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<()>;
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

    async fn object_create(&self, path: &str, content: &[u8], content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<()> {
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

    async fn object_get(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<Vec<u8>> {
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

    async fn object_exist(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<bool> {
        let bucket = self.get_bucket(bucket_name)?;
        let result = bucket.head_object(path).await.map_err(TardisError::from);
        if result.is_ok() {
            Ok(true)
        } else if result.clone().expect_err("").code == "404" {
            Ok(false)
        } else {
            Err(TardisError::custom(
                &result.expect_err("").code.clone(),
                &format!("[Tardis.OSClient] Failed to head object {}:{}", bucket.name, path),
                "-1-tardis-os-get-object-error",
            ))
        }
    }

    async fn object_delete(&self, path: &str, bucket_name: Option<&str>) -> TardisResult<()> {
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

    async fn object_copy(&self, from: &str, to: &str, bucket_name: Option<&str>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        bucket.copy_object_internal(urlencoding::encode(from), to).await?;
        Ok(())
    }

    async fn initiate_multipart_upload(&self, path: &str, content_type: Option<&str>, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.initiate_multipart_upload(path, content_type.unwrap_or("application/octet-stream")).await?.upload_id)
    }

    async fn batch_build_create_presign_url(&self, path: &str, upload_id: &str, part_number: u32, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<Vec<String>> {
        let mut presign_url = vec![];
        for part_no in 0..part_number {
            let mut custom_queries = HashMap::new();
            custom_queries.insert("uploadId".to_string(), upload_id.to_string());
            custom_queries.insert("partNumber".to_string(), (part_no + 1).to_string());
            presign_url.push(self.get_bucket(bucket_name)?.presign_put(path, expire_sec, None, Some(custom_queries)).await?);
        }
        Ok(presign_url)
    }

    async fn complete_multipart_upload(&self, path: &str, upload_id: &str, parts: Vec<String>, bucket_name: Option<&str>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        let mut part_params = vec![];
        for (i, etag) in parts.into_iter().enumerate() {
            part_params.push(Part {
                part_number: (i + 1) as u32,
                etag,
            });
        }
        bucket.complete_multipart_upload(path, upload_id, part_params).await?;

        Ok(())
    }

    async fn object_create_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_put(path, expire_sec, None, None).await?)
    }

    async fn object_get_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_get(path, expire_sec, None).await?)
    }

    async fn object_delete_url(&self, path: &str, expire_sec: u32, bucket_name: Option<&str>) -> TardisResult<String> {
        Ok(self.get_bucket(bucket_name)?.presign_delete(path, expire_sec).await?)
    }

    async fn get_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<BucketLifecycleConfiguration> {
        let bucket = self.get_bucket(bucket_name)?;
        Ok(bucket.get_bucket_lifecycle().await?)
    }

    async fn put_lifecycle(&self, bucket_name: Option<&str>, config: BucketLifecycleConfiguration) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        bucket.put_bucket_lifecycle(config).await?;
        Ok(())
    }

    async fn delete_lifecycle(&self, bucket_name: Option<&str>) -> TardisResult<()> {
        let bucket = self.get_bucket(bucket_name)?;
        bucket.delete_bucket_lifecycle().await?;
        Ok(())
    }
}

impl TardisOSS3Client {
    fn get_bucket(&self, bucket_name: Option<&str>) -> TardisResult<Box<Bucket>> {
        if let Some(bucket_name) = bucket_name {
            Ok(Bucket::new(bucket_name, self.region.clone(), self.credentials.clone())?.with_path_style())
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
            s3::error::S3Error::HttpFailWithBody(code, msg) => TardisError::custom(&code.to_string(), &format!("[Tardis.OSClient] Error: {}", msg), "-1-tardis-os-error"),
            _ => TardisError::custom(ERROR_DEFAULT_CODE, &format!("[Tardis.OSClient] Error: {error:?}"), "-1-tardis-os-error"),
        }
    }
}
