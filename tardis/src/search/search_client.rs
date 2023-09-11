use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, trace};
use url::Url;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::component_config::search::SearchModuleConfig;
use crate::config::component_config::SearchConfig;
use crate::config::config_dto::FrameworkConfig;
use crate::utils::initializer::InitBy;
use crate::{TardisFuns, TardisWebClient};

/// Distributed search handle / 分布式搜索操作
///
/// Encapsulates common elasticsearch operations.
///
/// 封装了Elasticsearch的常用操作.
///
/// # Steps to use / 使用步骤
///
/// 1. Create the search configuration / 创建搜索配置, @see [SearchConfig](crate::basic::config::SearchConfig)
///
/// 2. Use `TardisSearchClient` to operate search / 使用 `TardisSearchClient` 操作搜索, E.g:
/// ```ignore
/// use tardis::TardisFuns;
/// TardisFuns::search().create_index("test_index").await.unwrap();
/// let id = TardisFuns::search().create_record("test_index", r#"{"user":{"id":1,"name":"张三","open":false}}"#).await.unwrap();
/// assert_eq!(TardisFuns::search().get_record("test_index", &id).await.unwrap(), r#"{"user":{"id":4,"name":"Tom","open":true}}"#);
/// TardisFuns::search().simple_search("test_index", "张三").await.unwrap();
/// ```
pub struct TardisSearchClient {
    pub client: TardisWebClient,
    pub server_url: Url,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TardisRawSearchResp {
    pub hits: TardisRawSearchHits,
    pub took: i32,
    pub _shards: TardisRawSearchShards,
    pub timed_out: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TardisRawSearchHits {
    pub total: TardisRawSearchHitsTotal,
    pub hits: Vec<TardisRawSearchHitsItem>,
    pub max_score: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TardisRawSearchHitsTotal {
    pub value: i32,
    pub relation: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TardisRawSearchHitsItem {
    pub _index: String,
    pub _id: String,
    pub _score: Option<f32>,
    pub _source: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TardisRawSearchShards {
    pub failed: i32,
    pub successful: i32,
    pub total: i32,
}

#[async_trait::async_trait]
impl InitBy<SearchModuleConfig> for TardisSearchClient {
    async fn init(config: &SearchModuleConfig) -> TardisResult<Self> {
        Self::init(config)
    }
}

impl TardisSearchClient {
    /// Initialize configuration / 初始化配置
    pub fn init(SearchModuleConfig { url, timeout_sec }: &SearchModuleConfig) -> TardisResult<TardisSearchClient> {
        info!("[Tardis.SearchClient] Initializing");
        let mut client = TardisWebClient::init(*timeout_sec)?;
        client.set_default_header("Content-Type", "application/json");
        info!("[Tardis.SearchClient] Initialized");
        TardisResult::Ok(TardisSearchClient {
            client,
            server_url: url.clone(),
        })
    }

    /// Create index / 创建索引
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().create_index("test_index").await.unwrap();
    /// ```
    pub async fn create_index(&self, index_name: &str, mappings: Option<&str>) -> TardisResult<()> {
        trace!("[Tardis.SearchClient] Creating index: {}", index_name);
        let url = format!("{}/{}", self.server_url, index_name);
        let resp = self.client.put_str_to_str(&url, mappings.unwrap_or_default(), None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            Ok(())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Create index error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// Create record and return primary key value  / 创建记录并返回主键值
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `data` -  record content / 记录内容
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// let id = TardisFuns::search().create_record("test_index", r#"{"user":{"id":1,"name":"张三","open":false}}"#).await.unwrap();
    /// ```
    pub async fn create_record(&self, index_name: &str, data: &str) -> TardisResult<String> {
        trace!("[Tardis.SearchClient] Creating record: {}, data:{}", index_name, data);
        let url = format!("{}/{}/_doc/", self.server_url, index_name);
        let resp = self.client.post_str_to_str(&url, data, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            let result = TardisFuns::json.str_to_json(&resp.body.unwrap_or_default())?;
            Ok(result["_id"].as_str().ok_or_else(|| TardisError::bad_request("[Tardis.SearchClient] [_id] structure not found", "400-tardis-search-id-not-exist"))?.to_string())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Create record error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// Get a record  / 获取一条记录
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `id` -  record primary key value / 记录主键值
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().get_record("test_index", "xxxx").await.unwrap();
    /// ```
    pub async fn get_record(&self, index_name: &str, id: &str) -> TardisResult<String> {
        trace!("[Tardis.SearchClient] Getting record: {}, id:{}", index_name, id);
        let url = format!("{}/{}/_doc/{}", self.server_url, index_name, id);
        let resp = self.client.get_to_str(&url, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            let result = TardisFuns::json.str_to_json(&resp.body.unwrap_or_default())?;
            Ok(result["_source"].to_string())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Get record error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// Simple (global) search  / 简单（全局）搜索
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `q` -  keyword / 搜索关键字
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().simple_search("test_index", "张三").await.unwrap();
    /// ```
    pub async fn simple_search(&self, index_name: &str, q: &str) -> TardisResult<Vec<String>> {
        trace!("[Tardis.SearchClient] Simple search: {}, q:{}", index_name, q);
        let url = format!("{}/{}/_search?q={}", self.server_url, index_name, q);
        let resp = self.client.get_to_str(&url, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            Self::parse_search_result(&resp.body.unwrap_or_default())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Simple search error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// Specified fields search  / 指定字段搜索
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `q` -  search fields / 搜索的字段集合
    ///
    /// The format of the search field: key = field name , value = field value, exact match, key supports multi-level operations of Json.
    ///
    /// 搜索字段的格式: key = 字段名 , value = 字段值，精确匹配，key支持Json的多级操作.
    ///
    /// # Examples
    /// ```ignore
    /// use std::collections::HashMap;
    /// use tardis::TardisFuns;
    /// TardisFuns::search().multi_search(index_name, HashMap::from([("user.id", "1"), ("user.name", "李四")])).await.unwrap();
    /// ```
    pub async fn multi_search(&self, index_name: &str, q: HashMap<&str, &str>) -> TardisResult<Vec<String>> {
        trace!("[Tardis.SearchClient] Multi search: {}, q:{:?}", index_name, q);
        let q = q.into_iter().map(|(k, v)| format!(r#"{{"match": {{"{k}": "{v}"}}}}"#)).collect::<Vec<String>>().join(",");
        let q = format!(r#"{{ "query": {{ "bool": {{ "must": [{q}]}}}}}}"#);
        let result = self.raw_search(index_name, &q, None, None, None).await?.hits.hits.iter().map(|item| item._source.clone().to_string()).collect();
        Ok(result)
    }

    /// Search using native format  / 使用原生格式搜索
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `q` -  native format / 原生格式
    ///  * `size` -  number of shows / 展示的数量
    ///  * `from` -  offset / 偏移量
    ///  * `track_scores` -  calculating score / 计算相关性得分
    ///
    pub async fn raw_search(&self, index_name: &str, q: &str, size: Option<i32>, from: Option<i32>, track_scores: Option<bool>) -> TardisResult<TardisRawSearchResp> {
        trace!(
            "[Tardis.SearchClient] Raw search: {}, q:{}, size:{:?}, from:{:?}, track_scores:{:?}",
            index_name,
            q,
            size,
            from,
            track_scores
        );
        let mut url = format!("{}/{}/_search", self.server_url, index_name);
        let mut queries = vec![];

        if let Some(size) = size {
            queries.push(format!("size={}", size));
        }
        if let Some(from) = from {
            queries.push(format!("from={}", from));
        }
        if let Some(track_scores) = track_scores {
            queries.push(format!("track_scores={}", track_scores));
        }
        if !queries.is_empty() {
            url = format!("{}?{}", url, queries.join("&").as_str());
        }
        let resp = self.client.post_str_to_str(&url, q, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            trace!("[Tardis.SearchClient] resp.body: {:?}", &resp.body);
            Ok(TardisFuns::json.str_to_obj(&resp.body.unwrap_or_default())?)
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Raw search error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// check index exist  / 检查索引是否存在
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().check_index_exist("test_index").await.unwrap();
    /// ```
    pub async fn check_index_exist(&self, index_name: &str) -> TardisResult<bool> {
        trace!("[Tardis.SearchClient] Check index exist: {}", index_name);
        let url = format!("{}/{}", self.server_url, index_name);
        let resp = self.client.head_to_void(&url, None).await?;
        match resp.code {
            200 => Ok(true),
            404 => Ok(false),
            _ => Err(TardisError::custom(
                &resp.code.to_string(),
                "[Tardis.SearchClient] Check index exist request failed",
                "-1-tardis-search-error",
            )),
        }
    }

    /// update record / 更新记录
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `id` -  record primary key value / 记录主键值
    ///  * `q` -  native format / 原生格式
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().update("test_index", "111", HashMap::from([("user.id", "1"), ("user.name", "李四")])).await.unwrap();
    /// ```
    pub async fn update(&self, index_name: &str, id: &str, q: HashMap<String, String>) -> TardisResult<()> {
        let mut source_vec = vec![];
        let mut params_vec = vec![];
        for (key, value) in q {
            let param_key = key.replace('.', "_");
            source_vec.push(format!(r#"ctx._source.{key}= params.{param_key}"#));
            params_vec.push(format!(r#""{param_key}": {value}"#));
        }
        let source = source_vec.join(";");
        let params = params_vec.join(",");
        let q = format!(r#"{{ "script": {{"source": "{source}", "params":{{{params}}}}}}}"#);
        debug!("[Tardis.SearchClient] Update: {}, q:{}", index_name, q);
        let url = format!("{}/{}/_update/{}?refresh=true", self.server_url, index_name, id);
        let resp = self.client.post_str_to_str(&url, &q, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            trace!("[Tardis.SearchClient] resp.body: {:?}", &resp.body);
            Ok(())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Update error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    /// Delete record / 删除记录
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `q` -  native format / 原生格式
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// TardisFuns::search().delete_by_query("test_index" r#"{}"#).await.unwrap();
    /// ```
    pub async fn delete_by_query(&self, index_name: &str, q: &str) -> TardisResult<()> {
        let url = format!("{}/{}/_delete_by_query", self.server_url, index_name);
        let resp = self.client.post_str_to_str(&url, q, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            debug!("[Tardis.SearchClient] resp.body: {:?}", &resp.body);
            Ok(())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Delete by query error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
                "-1-tardis-search-error",
            ))
        }
    }

    fn parse_search_result(result: &str) -> TardisResult<Vec<String>> {
        let json = TardisFuns::json.str_to_json(result)?;
        let json = json["hits"]["hits"]
            .as_array()
            .ok_or_else(|| TardisError::format_error("[Tardis.SearchClient] [hit.hit] structure not found", "406-tardis-search-hit-not-exist"))?
            .iter()
            .map(|x| x["_source"].to_string())
            .collect();
        Ok(json)
    }
}
