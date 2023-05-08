use async_trait::async_trait;
use poem::{endpoint::BoxEndpoint, Endpoint, IntoResponse, Middleware, Request, Response, Route};

pub struct DefaultEmptyMW;

impl Middleware<Route> for DefaultEmptyMW {
    type Output = BoxEndpoint<'static>;

    fn transform(&self, ep: Route) -> Self::Output {
        Box::new(DefaultEmptyMWImpl(ep))
    }
}

pub struct DefaultEmptyMWImpl<E>(E);

#[async_trait]
impl<E: Endpoint> Endpoint for DefaultEmptyMWImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        match self.0.call(req).await {
            Ok(r) => Ok(r.into_response()),
            Err(r) => Err(r),
        }
    }
}
