use std::{collections::HashMap, net::SocketAddr};

use axum::{
    body::{self, Empty, Full},
    http::{HeaderMap, Method, Uri},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use once_cell::sync::Lazy;
use reqwest::{redirect::Policy, Client};

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/110.0")
        .redirect(Policy::limited(2))
        .build()
        .unwrap()
});

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", any(proxy));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap()
}

async fn proxy(method: Method, uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let params: HashMap<String, String> = uri
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);

    let host = params.get("host");

    if host.is_none() {
        return Response::builder()
            .status(400)
            .body(body::boxed(Empty::new()))
            .unwrap();
    }

    let target = format!("https://{}{}", host.unwrap(), uri.path_and_query().unwrap());
    println!("Url: {}", target);

    let request = CLIENT
        .request(method, target)
        .headers(headers)
        .build()
        .expect("Invalid request");

    let response = CLIENT.execute(request).await.expect("Invalid response");
    let headers = response.headers().clone();
    let status = response.status().clone();

    let body = response
        .bytes()
        .await
        .expect("Unable to read response body");
    let mut builder = Response::builder();

    for (key, value) in headers {
        if let Some(key) = key {
            builder = builder.header(key, value);
        }
    }

    return builder
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Max-Age", "1728000")
        .body(body::boxed(Full::from(body)))
        .unwrap();
}
