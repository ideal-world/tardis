use async_trait::async_trait;
use poem::{http::HeaderValue, Endpoint, IntoResponse, Middleware, Request, Response};

pub struct AddClusterIdHeader;

impl<E: Endpoint> Middleware<E> for AddClusterIdHeader {
    type Output = UniformErrorImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        UniformErrorImpl(ep)
    }
}

pub struct UniformErrorImpl<E>(E);
pub const TARDIS_CLUSTER_ID_HEADER: &str = "Tardis-Cluster-Id";
#[async_trait]
impl<E: Endpoint> Endpoint for UniformErrorImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let mut resp = self.0.call(req).await?.into_response();
        #[cfg(feature = "cluster")]
        {
            let cluster_id = crate::cluster::cluster_processor::local_node_id().await;
            if let Ok(header_value) = HeaderValue::from_str(cluster_id) {
                resp.headers_mut().insert(TARDIS_CLUSTER_ID_HEADER, header_value);
            }
        }
        Ok(resp)
    }
}