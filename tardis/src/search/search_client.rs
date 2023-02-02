use std::collections::HashMap;

use log::trace;

use crate::basic::error::TardisError;
use crate::basic::result::TardisResult;
use crate::config::config_dto::FrameworkConfig;
use crate::log::info;
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
    client: TardisWebClient,
    server_url: String,
}

impl TardisSearchClient {
    /// Initialize configuration from the search configuration object / 从搜索配置对象中初始化配置
    pub fn init_by_conf(conf: &FrameworkConfig) -> TardisResult<HashMap<String, TardisSearchClient>> {
        let mut clients = HashMap::new();
        clients.insert("".to_string(), TardisSearchClient::init(&conf.search.url, conf.search.timeout_sec)?);
        for (k, v) in &conf.search.modules {
            clients.insert(k.to_string(), TardisSearchClient::init(&v.url, v.timeout_sec)?);
        }
        Ok(clients)
    }

    /// Initialize configuration / 初始化配置
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
    pub async fn create_index(&self, index_name: &str) -> TardisResult<()> {
        trace!("[Tardis.SearchClient] Creating index: {}", index_name);
        let url = format!("{}/{}", self.server_url, index_name);
        let resp = self.client.put_str_to_str(&url, "", None).await?;
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
        self.raw_search(index_name, &q).await
    }

    /// Search using native format  / 使用原生格式搜索
    ///
    /// # Arguments
    ///
    ///  * `index_name` -  index name / 索引名称
    ///  * `q` -  native format / 原生格式
    ///
    pub async fn raw_search(&self, index_name: &str, q: &str) -> TardisResult<Vec<String>> {
        trace!("[Tardis.SearchClient] Raw search: {}, q:{}", index_name, q);
        let url = format!("{}/{}/_search", self.server_url, index_name);
        let resp = self.client.post_str_to_str(&url, q, None).await?;
        if resp.code >= 200 && resp.code <= 300 {
            Self::parse_search_result(&resp.body.unwrap_or_default())
        } else {
            Err(TardisError::custom(
                &resp.code.to_string(),
                &format!("[Tardis.SearchClient] Raw search error: {}", resp.body.as_ref().unwrap_or(&"".to_string())),
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
