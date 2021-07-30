use hyper::body::Buf;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use tokio::time::timeout;
use url::form_urlencoded::parse as url_parse;

use std::convert::Infallible;
use std::error::Error as StdError;
use std::io::Read;
use std::net::SocketAddr;
use std::result::Result as StdResult;
use std::time::Duration;

use crate::dns_client::DnsClient;

type GenericError = Box<dyn StdError>;

static MIME_TEXT: &[u8] = b"text/plain";
static MIME_DNS: &[u8] = b"application/dns-message";

async fn perform_dns_query_from_query(
    dns_client: &DnsClient,
    req: Request<Body>,
) -> Result<Response<Body>> {
    let request = match req
        .uri()
        .query()
        .and_then(|v| {
            url_parse(v.as_bytes())
                .filter(|(k, _v)| k == "dns")
                .map(|(_k, v)| v)
                .nth(0)
        })
        .map(|request| base64::decode_config(request.into_owned(), base64::URL_SAFE_NO_PAD))
    {
        Some(decoded) => match decoded {
            Ok(request) => request,
            Err(_err) => return Ok(response_bad_request(b"Could not decode the request")),
        },
        None => return Ok(response_bad_request(b"Missing the dns parameter")),
    };

    perform_dns_query(dns_client, &request).await
}

async fn perform_dns_query_from_body(
    dns_client: &DnsClient,
    req: Request<Body>,
) -> Result<Response<Body>> {
    let content_type = match req.headers().get(CONTENT_TYPE).map(|ct| ct.as_bytes()) {
        Some(ct) => ct,
        None => return Ok(response_bad_request(b"No content type")),
    };
    if content_type != MIME_DNS {
        return Ok(response_unsupported_content_type());
    }

    let request = match timeout(Duration::from_millis(1000), hyper::body::aggregate(req)).await {
        Ok(result) => match result {
            Ok(body) => {
                let mut data: Vec<u8> = Vec::new();
                data.resize_with(body.remaining(), || 0);
                body.reader().read(data.as_mut_slice()).unwrap();
                data
            }
            Err(_) => return Ok(response_bad_request(b"Network error")),
        },
        Err(_) => return Ok(response_timeout()),
    };

    perform_dns_query(dns_client, &request).await
}

async fn perform_dns_query(dns_client: &DnsClient, req: &[u8]) -> Result<Response<Body>> {
    let dns_response = match dns_client.call(req).await {
        Ok(response) => response,
        Err(_) => return Ok(response_server_error()),
    };

    Ok(response_ok(dns_response))
}

async fn router(req: Request<Body>, dns_client: DnsClient) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => perform_dns_query_from_query(&dns_client, req).await,
        (&Method::POST, "/") => perform_dns_query_from_body(&dns_client, req).await,
        _ => Ok(response_not_found()),
    }
}

/// HTTP status code 200
fn response_ok(body: Vec<u8>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, MIME_DNS)
        .header(CONTENT_LENGTH, body.len())
        .body(body.into())
        .unwrap()
}

/// HTTP status code 400
fn response_bad_request(body: &'static [u8]) -> Response<Body> {
    response_error2(StatusCode::BAD_REQUEST, body.into())
}

/// HTTP status code 404
fn response_not_found() -> Response<Body> {
    response_error(StatusCode::NOT_FOUND)
}

/// HTTP status code 408
fn response_timeout() -> Response<Body> {
    response_error(StatusCode::REQUEST_TIMEOUT)
}

/// HTTP status code 415
fn response_unsupported_content_type() -> Response<Body> {
    response_error2(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        b"Unsupported content type",
    )
}

/// HTTP status code 500
fn response_server_error() -> Response<Body> {
    response_error(StatusCode::INTERNAL_SERVER_ERROR)
}

fn response_error(status_code: StatusCode) -> Response<Body> {
    response_error2(
        status_code,
        status_code.canonical_reason().unwrap().as_bytes(),
    )
}

fn response_error2(status_code: StatusCode, body: &'static [u8]) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .header(CONTENT_TYPE, MIME_TEXT)
        .header(CONTENT_LENGTH, body.len())
        .body(body.into())
        .unwrap()
}

pub async fn run(listen: &SocketAddr, dns_client: DnsClient) -> StdResult<(), GenericError> {
    let make_svc = make_service_fn(move |_| {
        let dns_client = dns_client.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| router(req, dns_client.clone()))) }
    });

    if let Err(e) = Server::bind(listen).serve(make_svc).await {
        return Err(Box::new(e));
    }
    Ok(())
}
