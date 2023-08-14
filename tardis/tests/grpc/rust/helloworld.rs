/// The request message containing the user's name.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
/// The response message containing the greetings
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloReply {
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
}
#[allow(unused_imports)]
#[derive(Clone)]
pub struct GreeterClient {
    cli: poem_grpc::client::GrpcClient,
}
#[allow(dead_code)]
impl GreeterClient {
    #[allow(clippy::let_and_return)]
    pub fn new(config: poem_grpc::ClientConfig) -> Self {
        Self {
            cli: {
                let cli = poem_grpc::client::GrpcClient::new(config);
                cli
            },
        }
    }
    #[allow(clippy::let_and_return)]
    pub fn from_endpoint<T>(ep: T) -> Self
    where
        T: ::poem::IntoEndpoint,
        T::Endpoint: 'static,
        <T::Endpoint as ::poem::Endpoint>::Output: 'static,
    {
        Self {
            cli: {
                let cli = poem_grpc::client::GrpcClient::from_endpoint(ep);
                cli
            },
        }
    }
    pub fn with<M>(mut self, middleware: M) -> Self
    where
        M: ::poem::Middleware<
            ::std::sync::Arc<dyn ::poem::Endpoint<Output = ::poem::Response> + 'static>,
        >,
        M::Output: 'static,
    {
        self.cli = self.cli.with(middleware);
        self
    }
    #[allow(dead_code)]
    pub async fn say_hello(
        &self,
        request: poem_grpc::Request<HelloRequest>,
    ) -> ::std::result::Result<poem_grpc::Response<HelloReply>, poem_grpc::Status> {
        let codec = <poem_grpc::codec::ProstCodec<
            _,
            _,
        > as ::std::default::Default>::default();
        self.cli.unary("/helloworld.Greeter/SayHello", codec, request).await
    }
}
#[allow(unused_imports)]
#[::poem::async_trait]
pub trait Greeter: Send + Sync + 'static {
    async fn say_hello(
        &self,
        request: poem_grpc::Request<HelloRequest>,
    ) -> ::std::result::Result<poem_grpc::Response<HelloReply>, poem_grpc::Status>;
}
#[allow(unused_imports)]
#[derive(Clone)]
pub struct GreeterServer<T>(::std::sync::Arc<T>);
impl<T: Greeter> poem_grpc::Service for GreeterServer<T> {
    const NAME: &'static str = "helloworld.Greeter";
}
#[allow(dead_code)]
impl<T> GreeterServer<T> {
    pub fn new(service: T) -> Self {
        Self(::std::sync::Arc::new(service))
    }
}
impl<T: Greeter> ::poem::IntoEndpoint for GreeterServer<T> {
    type Endpoint = ::poem::endpoint::BoxEndpoint<'static, ::poem::Response>;
    #[allow(clippy::redundant_clone)]
    #[allow(clippy::let_and_return)]
    fn into_endpoint(self) -> Self::Endpoint {
        use ::poem::endpoint::EndpointExt;
        let mut route = ::poem::Route::new();
        #[allow(non_camel_case_types)]
        struct Greetersay_helloService<T>(::std::sync::Arc<T>);
        #[::poem::async_trait]
        impl<T: Greeter> poem_grpc::service::UnaryService<HelloRequest>
        for Greetersay_helloService<T> {
            type Response = HelloReply;
            async fn call(
                &self,
                request: poem_grpc::Request<HelloRequest>,
            ) -> Result<poem_grpc::Response<Self::Response>, poem_grpc::Status> {
                self.0.say_hello(request).await
            }
        }
        route = route
            .at(
                "/SayHello",
                ::poem::endpoint::make({
                    let svc = self.0.clone();
                    move |req| {
                        let svc = svc.clone();
                        async move {
                            let codec = <poem_grpc::codec::ProstCodec<
                                _,
                                _,
                            > as ::std::default::Default>::default();
                            poem_grpc::server::GrpcServer::new(codec)
                                .unary(Greetersay_helloService(svc.clone()), req)
                                .await
                        }
                    }
                }),
            );
        let ep = route
            .before(|req| async move {
                if req.version() != ::poem::http::Version::HTTP_2 {
                    return Err(
                        ::poem::Error::from_status(
                            ::poem::http::StatusCode::HTTP_VERSION_NOT_SUPPORTED,
                        ),
                    );
                }
                Ok(req)
            });
        ep.boxed()
    }
}
