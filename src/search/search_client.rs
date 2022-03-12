use std::collections::HashMap;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::log::{debug, info};
use crate::{FrameworkConfig, TardisFuns, TardisWebClient};

pub struct TardisSearchClient {
    client: TardisWebClient,
    server_url: String,
}

impl TardisSearchClient {
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<TardisSearchClient> {
        TardisSearchClient::init(&conf.search.url, conf.search.timeout_sec)
    }

    pub fn init(str_url: &str, timeout_sec: u64) -> TardisResult<TardisSearchClient> {
        info!("[Tardis.SearchClient] Initializing");
        let mut client = TardisWebClient::init(timeout_sec)?;
        client.set_default_header("Content-Type", "application/json");
        info!("[Tardis.SearchClient] Initialized");
        TardisResult::Ok(TardisSearchClient {
            client,
            server_url: str_url.to_string(),
        })
    }

    pub async fn create_index(&self, index_name: &str) -> TardisResult<()> {
        info!("[Tardis.SearchClient] Create index {}", index_name);
        let url = format!("{}/{}", self.server_url, index_name);
        let resp = self.client.put_str_to_str(&url, "", None).await?;
        if let Some(err) = TardisError::new(resp.code, resp.body.as_ref().unwrap_or(&"".to_string())) {
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn create_record(&self, index_name: &str, data: &str) -> TardisResult<String> {
        debug!("[Tardis.SearchClient] Create index {}", index_name);
        let url = format!("{}/{}/_doc/", self.server_url, index_name);
        let resp = self.client.post_str_to_str(&url, data, None).await?;
        if let Some(err) = TardisError::new(resp.code, resp.body.as_ref().unwrap_or(&"".to_string())) {
            Err(err)
        } else {
            let result = TardisFuns::json.str_to_json(&resp.body.unwrap_or_else(|| "".to_string()))?;
            Ok(result["_id"].as_str().ok_or_else(|| TardisError::FormatError("[Tardis.SearchClient] [_id] structure not found".to_string()))?.to_string())
        }
    }

    pub async fn get_record(&self, index_name: &str, id: &str) -> TardisResult<String> {
        let url = format!("{}/{}/_doc/{}", self.server_url, index_name, id);
        let resp = self.client.get_to_str(&url, None).await?;
        if let Some(err) = TardisError::new(resp.code, resp.body.as_ref().unwrap_or(&"".to_string())) {
            Err(err)
        } else {
            let result = TardisFuns::json.str_to_json(&resp.body.unwrap_or_else(|| "".to_string()))?;
            Ok(result["_source"].to_string())
        }
    }

    pub async fn simple_search(&self, index_name: &str, q: &str) -> TardisResult<Vec<String>> {
        let url = format!("{}/{}/_search?q={}", self.server_url, index_name, q);
        let resp = self.client.get_to_str(&url, None).await?;
        if let Some(err) = TardisError::new(resp.code, resp.body.as_ref().unwrap_or(&"".to_string())) {
            Err(err)
        } else {
            Self::parse_search_result(&resp.body.unwrap_or_else(|| "".to_string()))
        }
    }

    pub async fn multi_search(&self, index_name: &str, q: HashMap<&str, &str>) -> TardisResult<Vec<String>> {
        let q = q.into_iter().map(|(k, v)| format!(r#"{{"match": {{"{}": "{}"}}}}"#, k, v)).collect::<Vec<String>>().join(",");
        let q = format!(r#"{{ "query": {{ "bool": {{ "must": [{}]}}}}}}"#, q);
        self.raw_search(index_name, &q).await
    }

    pub async fn raw_search(&self, index_name: &str, q: &str) -> TardisResult<Vec<String>> {
        let url = format!("{}/{}/_search", self.server_url, index_name);
        let resp = self.client.post_str_to_str(&url, q, None).await?;
        if let Some(err) = TardisError::new(resp.code, resp.body.as_ref().unwrap_or(&"".to_string())) {
            Err(err)
        } else {
            Self::parse_search_result(&resp.body.unwrap_or_else(|| "".to_string()))
        }
    }

    fn parse_search_result(result: &str) -> TardisResult<Vec<String>> {
        let json = TardisFuns::json.str_to_json(result)?;
        let json = json["hits"]["hits"]
            .as_array()
            .ok_or_else(|| TardisError::FormatError("[Tardis.SearchClient] [hit.hit] structure not found".to_string()))?
            .iter()
            .map(|x| x["_source"].to_string())
            .collect();
        Ok(json)
    }
}
