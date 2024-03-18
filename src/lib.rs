use std::{
    convert::Infallible,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A marker trait that indicates that an extractor is compatible with a particular request
pub trait FromRequest<Query, Body> {}

impl<'de, T: Serialize + Deserialize<'de>, Query> FromRequest<Query, JsonBody<T>>
    for actix_web::web::Json<T>
{
}

impl<'de, T: Serialize + Deserialize<'de>, Body> FromRequest<Query<T>, Body>
    for actix_web::web::Query<T>
{
}

macro_rules! impl_from_request {
    ($($i:ident)*) => {
        impl<Query, Body $(,$i)*> FromRequest<Query, Body> for ($($i,)*)
        where
            $($i: FromRequest<Query, Body>),*

         {}
    };
}

impl_from_request! {}
impl_from_request! { A }
impl_from_request! { A B }
impl_from_request! { A B C }
impl_from_request! { A B C D }
impl_from_request! { A B C D E }
impl_from_request! { A B C D E F }
impl_from_request! { A B C D E F G }
impl_from_request! { A B C D E F G H }
impl_from_request! { A B C D E F G H I }
impl_from_request! { A B C D E F G H I J }
impl_from_request! { A B C D E F G H I J K }
impl_from_request! { A B C D E F G H I J K L }
impl_from_request! { A B C D E F G H I J K L M }
impl_from_request! { A B C D E F G H I J K L M N }
impl_from_request! { A B C D E F G H I J K L M N O }
impl_from_request! { A B C D E F G H I J K L M N O P }

/// A marker trait that indicates that a return type is compatible with a particular response
pub trait IntoResponse<Body> {}

impl<T> IntoResponse<NoBody> for T {}

impl<R, T: IntoResponse<JsonBody<R>>, E> IntoResponse<JsonBody<R>> for Result<T, E> {}

/// A type that indicates that the request makes no guarantees about its query string.
pub struct NoQuery;

/// A type that indicates that the request guarantees that its query string will successfully
/// deserialize into a `T`.
///
/// This will use [serde_urlencoded] to serialize to and deserialize from the query string.
pub struct Query<T>(T);

/// A type that indicates that the request or response makes no guarantees about its body,
/// or if it even has one.
pub struct NoBody;

/// A type that indicates that the request or response guarantees that its body will be JSON
/// that successfully deserializes into a `T` when using `serde_json`'s deserializer.
pub struct JsonBody<T>(T);

/// An extractor wrapper that opts-out of checking if the inner extractor is compatible with the request.
/// Useful if you want to use a third-party extractor that doesn't implement [FromRequest].
///
/// For your own extractors you should favour implementing [FromRequest] on it over using this.
pub struct NoCheck<T>(pub T);

impl<T> Deref for NoCheck<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoCheck<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

trait ApplyToRequestHead {
    type Error;
    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::RequestBuilder, Self::Error>;
}

impl ApplyToRequestHead for NoQuery {
    type Error = Infallible;
    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::RequestBuilder, Self::Error> {
        Ok(builder)
    }
}

impl<T: Serialize> ApplyToRequestHead for Query<T> {
    type Error = serde_urlencoded::ser::Error;
    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::RequestBuilder, Self::Error> {
        let params = serde_urlencoded::to_string(self.0)?;
        let params = params.split('&').filter_map(|pair| pair.split_once('='));
        Ok(builder.query(params))
    }
}

trait ApplyToRequestBody {
    type Error;
    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::Request, Self::Error>;
}

impl ApplyToRequestBody for NoBody {
    type Error = gloo_net::Error;
    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::Request, Self::Error> {
        builder.build()
    }
}
impl<T: Serialize> ApplyToRequestBody for JsonBody<T> {
    type Error = gloo_net::Error;

    fn apply(
        self,
        builder: gloo_net::http::RequestBuilder,
    ) -> Result<gloo_net::http::Request, Self::Error> {
        builder.json(&self.0)
    }
}

pub trait Route {
    type Query: ApplyToRequestHead;

    type RequestBody: ApplyToRequestBody;

    type ResponseBody;

    const METHOD: http::Method;

    const URI_PART: &'static str;
    const URI: &'static str;
}

struct RequestBuilder<Route, Query, Body> {
    _marker: PhantomData<*const Route>,
    query: Query,
    body: Body,
    builder: gloo_net::http::RequestBuilder,
}

impl<Route: self::Route> RequestBuilder<Route, NoQuery, NoBody> {
    pub fn new() -> Self {
        let builder = gloo_net::http::RequestBuilder::new(Route::URI).method(match Route::METHOD {
            http::Method::GET => gloo_net::http::Method::GET,
            http::Method::POST => gloo_net::http::Method::POST,
            http::Method::PUT => gloo_net::http::Method::PUT,
            http::Method::DELETE => gloo_net::http::Method::DELETE,
            http::Method::HEAD => gloo_net::http::Method::HEAD,
            http::Method::OPTIONS => gloo_net::http::Method::OPTIONS,
            http::Method::CONNECT => gloo_net::http::Method::CONNECT,
            http::Method::PATCH => gloo_net::http::Method::PATCH,
            http::Method::TRACE => gloo_net::http::Method::TRACE,
            _ => unimplemented!(),
        });
        Self {
            _marker: PhantomData,
            query: NoQuery,
            body: NoBody,
            builder,
        }
    }
}

impl<Route: self::Route, Query, Body> RequestBuilder<Route, Query, Body> {
    /// Provide additional query parameters that are not required by the route definition.
    pub fn extra_query<'a, T, V>(mut self, params: T) -> Self
    where
        T: IntoIterator<Item = (&'a str, V)>,
        V: AsRef<str>,
    {
        self.builder = self.builder.query(params);
        self
    }
}

impl<T, Route: self::Route<Query = Query<T>>, Body> RequestBuilder<Route, NoQuery, Body> {
    pub fn query(self, query: T) -> RequestBuilder<Route, Query<T>, Body> {
        RequestBuilder {
            _marker: self._marker,
            query: Query(query),
            body: self.body,
            builder: self.builder,
        }
    }
}

impl<T, Route: self::Route<RequestBody = JsonBody<T>>, Query> RequestBuilder<Route, Query, NoBody> {
    pub fn json(self, json: T) -> RequestBuilder<Route, Query, JsonBody<T>> {
        RequestBuilder {
            _marker: self._marker,
            query: self.query,
            body: JsonBody(json),
            builder: self.builder,
        }
    }
}

#[derive(Debug, Error)]
enum RequestBuildError<QueryError, BodyError> {
    #[error("Failed to build query")]
    QueryError(#[source] QueryError),
    #[error("Failed to build body")]
    BodyError(#[source] BodyError),
}

impl<
        Query: ApplyToRequestHead,
        Body: ApplyToRequestBody,
        Route: self::Route<Query = Query, RequestBody = Body>,
    > RequestBuilder<Route, Query, Body>
{
    fn build(
        self,
    ) -> Result<
        Request<Route>,
        RequestBuildError<
            <Route::Query as ApplyToRequestHead>::Error,
            <Route::RequestBody as ApplyToRequestBody>::Error,
        >,
    > {
        let builder = match self.query.apply(self.builder) {
            Ok(builder) => builder,
            Err(query_error) => return Err(RequestBuildError::QueryError(query_error)),
        };

        let request = match self.body.apply(builder) {
            Ok(request) => request,
            Err(body_error) => return Err(RequestBuildError::BodyError(body_error)),
        };

        Ok(Request {
            _marker: PhantomData,
            request,
        })
    }
}

struct Request<Route> {
    _marker: PhantomData<*const Route>,
    request: gloo_net::http::Request,
}

impl<Route: self::Route> Request<Route> {
    pub async fn send(self) -> Result<Response<Route>, gloo_net::Error> {
        self.request.send().await.map(|response| Response {
            _marker: PhantomData,
            response,
        })
    }
}

struct Response<Route> {
    _marker: PhantomData<*const Route>,
    response: gloo_net::http::Response,
}

impl<Route: self::Route> Response<Route> {
    pub fn status(&self) -> u16 {
        self.response.status()
    }

    pub fn ok(&self) -> bool {
        self.response.ok()
    }

    pub fn headers(&self) -> gloo_net::http::Headers {
        self.response.headers()
    }

    pub fn body_used(&self) -> bool {
        self.response.body_used()
    }

    pub fn into_untyped_response(self) -> gloo_net::http::Response {
        self.response
    }
}

impl<T: for<'de> serde::Deserialize<'de>, Route: self::Route<ResponseBody = JsonBody<T>>>
    Response<Route>
{
    pub async fn json(&self) -> Result<T, gloo_net::Error> {
        self.response.json().await
    }
}

pub struct Handled<Route, F> {
    _marker: PhantomData<*const Route>,
    handler: F,
}

pub fn handled_by<Route, Args, F>(f: F) -> Handled<Route, F>
where
    Route: self::Route,
    Args: FromRequest<Route::Query, Route::RequestBody>,
    F: actix_web::Handler<Args>,
    F::Output: IntoResponse<Route::ResponseBody>,
{
    Handled {
        _marker: PhantomData,
        handler: f,
    }
}

pub trait Router {
    fn app_data<U: 'static>(self, ext: U) -> Self;
    fn configure<F: FnOnce(&mut actix_web::web::ServiceConfig)>(self, f: F) -> Self;
    fn default_service<F, U>(self, svc: F) -> Self
    where
        F: actix_service::IntoServiceFactory<U, actix_web::dev::ServiceRequest>,
        U: actix_service::ServiceFactory<
                actix_web::dev::ServiceRequest,
                Config = (),
                Response = actix_web::dev::ServiceResponse,
                Error = actix_web::Error,
            > + 'static,
        U::InitError: std::fmt::Debug;
    fn route(self, path: &str, route: actix_web::Route) -> Self;
    fn service<F: actix_web::dev::HttpServiceFactory + 'static>(self, factory: F) -> Self;
}

impl<
        T: actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Error = actix_web::Error,
            InitError = (),
        >,
    > Router for actix_web::App<T>
{
    fn route(self, path: &str, route: actix_web::Route) -> Self {
        self.route(path, route)
    }

    fn service<F: actix_web::dev::HttpServiceFactory + 'static>(self, factory: F) -> Self {
        self.service(factory)
    }

    fn app_data<U: 'static>(self, ext: U) -> Self {
        self.app_data(ext)
    }

    fn configure<F: FnOnce(&mut actix_web::web::ServiceConfig)>(self, f: F) -> Self {
        self.configure(f)
    }

    fn default_service<F, U>(self, svc: F) -> Self
    where
        F: actix_service::IntoServiceFactory<U, actix_web::dev::ServiceRequest>,
        U: actix_service::ServiceFactory<
                actix_web::dev::ServiceRequest,
                Config = (),
                Response = actix_web::dev::ServiceResponse,
                Error = actix_web::Error,
            > + 'static,
        U::InitError: std::fmt::Debug,
    {
        self.default_service(svc)
    }
}

impl<
        T: actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Error = actix_web::Error,
            InitError = (),
        >,
    > Router for actix_web::Scope<T>
{
    fn route(self, path: &str, route: actix_web::Route) -> Self {
        self.route(path, route)
    }

    fn service<F: actix_web::dev::HttpServiceFactory + 'static>(self, factory: F) -> Self {
        self.service(factory)
    }

    fn app_data<U: 'static>(self, ext: U) -> Self {
        self.app_data(ext)
    }

    fn configure<F: FnOnce(&mut actix_web::web::ServiceConfig)>(self, f: F) -> Self {
        self.configure(f)
    }

    fn default_service<F, U>(self, svc: F) -> Self
    where
        F: actix_service::IntoServiceFactory<U, actix_web::dev::ServiceRequest>,
        U: actix_service::ServiceFactory<
                actix_web::dev::ServiceRequest,
                Config = (),
                Response = actix_web::dev::ServiceResponse,
                Error = actix_web::Error,
            > + 'static,
        U::InitError: std::fmt::Debug,
    {
        self.default_service(svc)
    }
}

pub trait Module {
    fn register<R: Router>(self, router: R) -> R;
}

#[macro_export]
macro_rules! routes {
    {
        module: type $module:ident;
        $(
            outer_routes: type $outer_routes_type:ident [
                $(route($($outer_route:tt)*)),*
                $(,)?
            ];
        )?
        scope: $uri:expr;
        inner_items: type $inner_routes_type:ident [
            $(($($inner_item:tt)*)),*
            $(,)?
        ];
    } => {
        $(
            outer_routes_typedef!{$outer_routes_type { $($($outer_route)*),*}}
        )?

        pub struct $module;

        impl $crate::Module for $module {
            fn register<R: $crate::Router>(self, router: R) -> R {

            }
        }

        const URI: &'static str = const_str::concat!(super::URI,$uri);
        $($(
            define_route_type!($($outer_route)*);
        )*)?
    };
}

macro_rules! define_item {
    (route($($route:tt)*)) => {
        define_route_type!($($route)*)
    };
}

#[macro_export]
macro_rules! define_route_type {
    ($method:expr, $uri_part:expr => type $type_name:ident (query: $query_type:ty, body: $body_type:ty $(,)?) -> $response_type:ty) => {
        pub struct $type_name;
        impl crate::Route for $type_name {
            type Query = $query_type;
            type RequestBody = $body_type;
            type ResponseBody = $response_type;
            const METHOD: http::Method = $method;
            const URI_PART: &'static str = $uri_part;
            const URI: &'static str = const_str::concat!(super::URI, $uri_part);
        }
    };
}


macro_rules! outer_routes_typedef {
    ($outer_routes_type:ident {$($method:expr, $uri_part:expr => type $type_name:ident (query: $query_type:ty, body: $body_type:ty $(,)?) -> $response_type:ty),*}) => {
        
    };
}

const URI: &'static str = "/123";

mod x {
    use crate::routes;
    use crate::{JsonBody, NoBody, NoQuery, Query};
    use http::Method;

    pub struct AbcRequest;

    routes! {
        module: type Module;
        outer_routes: type ModuleOuter [
            route(Method::POST, "/abc" => type Abc (query: NoQuery, body: JsonBody<Vec<u8>>) -> JsonBody<(String, u8)>)
        ];
        scope: "/xyz";
        inner_items: type ModuleInner [];
    }
}

async fn f() {
    let x = Default::default();
    let r = RequestBuilder::<x::Abc, _, _>::new()
        .json(x)
        .build()
        .unwrap()
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
}
