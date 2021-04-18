//! A warp filter for accepting form submissions using any HTTP method.
//!
//! [![Crates.io](https://img.shields.io/crates/v/warp_form_method.svg)](https://crates.io/crates/warp_form_method)
//! [![Docs.rs](https://docs.rs/warp_form_method/badge.svg)](https://docs.rs/warp_form_method/)
//! [![Crates.io](https://img.shields.io/crates/l/warp_form_method.svg)](https://github.com/maxdeviant/warp-form-method/blob/master/LICENSE)
//!
//! ## Installation
//! ```toml
//! [dependencies]
//! warp_form_method = "0.1"
//! ```

#![warn(missing_docs)]

use std::convert::TryFrom;

use futures::future;
use warp::http::Method;
use warp::{Buf, Filter};

/// Returns a [`Filter`] that matches a request with the following criteria:
/// - is a `POST` request
/// - has a `Content-Type: application/x-www-form-urlencoded` header and body
/// - the first field in the form has the name `_method` and a valid HTTP method
/// as the value
/// - the value of the `_method` field matches the specified HTTP method
///
/// Typically HTML forms can only be submitted as `GET` or `POST` requests, so
/// this filter allows for submitting forms using any HTTP method.
pub fn form_method(method: Method) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::post()
        .and(is_form_content())
        .and(warp::body::aggregate())
        .map(parse_method_in_first_field)
        .and_then(move |form_method| match form_method {
            Some(form_method) if form_method == method => future::ok(()),
            _ => future::err(warp::reject()),
        })
        .untuple_one()
}

fn is_form_content() -> impl Filter<Extract = (), Error = warp::Rejection> + Copy {
    warp::header::exact_ignore_case("Content-Type", "application/x-www-form-urlencoded")
}

/// The minimum length of the `_method` field.
const MIN_LEN: usize = "_method=GET".len();

/// The maximum length of the `_method` field.
const MAX_LEN: usize = "_method=DELETE".len();

/// Attempts to parse a `_method` field containing an HTTP method as the
/// **first** field in an `application/x-www-form-urlencoded` body.
///
/// If the `_method` field is not present, not the first field, or contains a
/// value that can not be parsed as an HTTP method this will return [`None`].
fn parse_method_in_first_field(mut body: impl Buf) -> Option<Method> {
    if body.remaining() < MIN_LEN {
        return None;
    }

    let mut peek_buffer = vec![0; std::cmp::min(body.remaining(), MAX_LEN)];
    body.copy_to_slice(&mut peek_buffer);

    let mut parts = std::str::from_utf8(&peek_buffer)
        .ok()?
        .split(|c| c == '=' || c == '&')
        .take(2);

    let name = parts.next();
    let value = parts.next();
    match (name, value) {
        (Some("_method"), Some(value)) => Method::try_from(value).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_matches_with_post_method_form_content_and_matching_put_method_in_first_field() {
        let filter = form_method(Method::PUT);

        assert!(
            warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=PUT&first_name=john")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_matches_with_post_method_form_content_and_matching_delete_method_in_first_field() {
        let filter = form_method(Method::DELETE);

        assert!(
            warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=DELETE&first_name=john")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_matches_with_the_minimum_form_body_length() {
        let filter = form_method(Method::PUT);

        assert!(
            warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=PUT")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_matches_with_a_form_body_length_between_the_minimum_and_maximum() {
        let filter = form_method(Method::HEAD);

        assert!(
            warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=HEAD")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_matches_with_the_maximum_form_body_length() {
        let filter = form_method(Method::DELETE);

        assert!(
            warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=DELETE")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_rejects_with_post_method_form_content_and_matching_method_not_in_first_field() {
        let filter = form_method(Method::PUT);

        assert!(
            !warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("first_name=john&_method=PUT")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_rejects_with_post_method_form_content_and_different_method_in_first_field() {
        let filter = form_method(Method::PUT);

        assert!(
            !warp::test::request()
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body("_method=DELETE&first_name=john")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_rejects_with_post_method_form_content_and_no_content_type_header() {
        let filter = form_method(Method::PUT);

        assert!(
            !warp::test::request()
                .method("POST")
                .body("_method=PUT&first_name=john")
                .matches(&filter)
                .await
        )
    }

    #[tokio::test]
    async fn it_rejects_with_post_method_and_no_content() {
        let filter = form_method(Method::PUT);

        assert!(!warp::test::request().method("POST").matches(&filter).await)
    }
}
