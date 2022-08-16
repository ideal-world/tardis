use crate::basic::result::TardisResult;

/// Uri handle / Uri处理
///
/// # Examples
/// ```ignore
/// use tardis::TardisFuns;
/// assert_eq!(TardisFuns::uri.format("http://idealwrold.group").unwrap(), "http://idealwrold.group");
/// assert_eq!(TardisFuns::uri.format("jdbc:h2:men:iam").unwrap(), "jdbc:h2:men:iam");
/// assert_eq!(TardisFuns::uri.format("api://a1.t1/e1?q2=2&q1=1&q3=3").unwrap(), "api://a1.t1/e1?q1=1&q2=2&q3=3");
/// ```
pub struct TardisUri;

impl TardisUri {
    /// Format Uri / 格式化Uri
    ///
    /// Return the standard, Query parameter sorted Uri.
    ///
    /// 返回标准的、Query参数排序后的Uri.
    ///
    /// # Arguments
    ///
    /// * `host` - Host
    /// * `path_and_query` - Path and Query
    pub fn format_with_item(&self, host: &str, path_and_query: &str) -> TardisResult<String> {
        if path_and_query.is_empty() {
            self.format(host)
        } else if path_and_query.starts_with('/') && !host.ends_with('/') || !path_and_query.starts_with('/') && host.ends_with('/') {
            self.format(format!("{}{}", host, path_and_query).as_str())
        } else if path_and_query.starts_with('/') && host.ends_with('/') {
            self.format(format!("{}/{}", host, path_and_query).as_str())
        } else {
            self.format(format!("{}/{}", host, &path_and_query[1..]).as_str())
        }
    }

    /// Format Uri / 格式化Uri
    ///
    /// Return the standard, Query parameter sorted Uri.
    ///
    /// 返回标准的、Query参数排序后的Uri.
    ///
    /// # Arguments
    ///
    /// * `uri_str` - Uri string
    ///
    /// # Examples
    /// ```ignore
    /// use tardis::TardisFuns;
    /// assert_eq!(TardisFuns::uri.format("api://a1.t1/e1?q2=2&q1=1&q3=3").unwrap(), "api://a1.t1/e1?q1=1&q2=2&q3=3");
    /// ```
    pub fn format(&self, uri_str: &str) -> TardisResult<String> {
        let uri = url::Url::parse(uri_str)?;
        let authority = if let Some(password) = uri.password() {
            format!("{}:{}@", uri.username(), password)
        } else if !uri.username().is_empty() {
            format!("{}@", uri.username())
        } else {
            String::new()
        };
        let host = match uri.host() {
            Some(host) => host,
            None =>
            // E.g. jdbc:h2:men:iam 不用解析
            {
                return Ok(uri.to_string())
            }
        };
        let port = match uri.port() {
            Some(port) => format!(":{}", port),
            None => "".to_string(),
        };
        let path = if uri.path().is_empty() {
            ""
        } else if uri.path().ends_with('/') {
            &uri.path()[..uri.path().len() - 1]
        } else {
            uri.path()
        };
        let query = self.sort_query(uri.query());
        let query = match uri.query() {
            Some(_) => format!("?{}", query),
            None => "".to_string(),
        };
        let formatted_uri = format!("{}://{}{}{}{}{}", uri.scheme(), authority, host, port, path, query);
        Ok(formatted_uri)
    }

    /// Get the Path and Query parts of the Uri / 获取Uri中的Path和Query部分
    ///
    /// # Arguments
    ///
    /// * `uri_str` - Uri string
    ///
    pub fn get_path_and_query(&self, uri_str: &str) -> TardisResult<String> {
        let uri = url::Url::parse(uri_str)?;
        let path = if uri.path().is_empty() {
            ""
        } else if uri.path().ends_with('/') {
            &uri.path()[..uri.path().len() - 1]
        } else {
            uri.path()
        };
        let query = match uri.query() {
            None => "".to_string(),
            Some(q) => format!("?{}", q),
        };
        return Ok(format!("{}{}", path, query));
    }

    fn sort_query(&self, query: Option<&str>) -> String {
        match query {
            None => "".to_string(),
            Some(query) => {
                let mut query = query.split('&').collect::<Vec<&str>>();
                query.sort_by(|a, b| Ord::cmp(a.split('=').next().unwrap_or(""), b.split('=').next().unwrap_or("")));
                query.join("&")
            }
        }
    }
}
