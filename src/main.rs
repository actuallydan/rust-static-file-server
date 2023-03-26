use async_trait::async_trait;
use axum::{
    body::StreamBody,
    extract::{FromRequestParts, Path},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use http_range_header::ParsedRanges;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

struct RangesHeader(ParsedRanges);

#[async_trait]
impl<S> FromRequestParts<S> for RangesHeader
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let range_header_str = parts
            .headers
            .get(header::RANGE)
            .ok_or((StatusCode::BAD_REQUEST, "`Range` header is missing"))?
            .to_str()
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "`Authorization` header contains invalid characters",
                )
            })?;

        let ranges = http_range_header::parse_range_header(range_header_str).unwrap();

        Ok(Self(ranges))
    }
}

fn read_files_from_dir(dir: &str) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.append(&mut read_files_from_dir(&path.to_str().unwrap())?);
        } else {
            files.push(path.to_str().unwrap().to_string());
        }
    }
    Ok(files)
}

// basic handler that responds with a static string
async fn root() -> String {
    let files = read_files_from_dir("src").unwrap();
    String::from(&files.join(",").to_string())
}

//handler that takes an `id` parameter and returns the filename of that index in the directory, otherwise returns a 404 string
async fn get_file(Path(id): Path<u64>) -> Response {
    // get parameters from the request

    let files = read_files_from_dir("videos").unwrap();
    println!("id: {}", id.to_string());

    if files.len() > id as usize {
        let file_name = String::from(&files[id as usize]);
        let file_type = &file_name.split(".").last().unwrap();
        let file = File::open(&file_name).await.unwrap();

        let meta = file.metadata().await.unwrap();

        // if let Some(RangesHeader(parsed_ranges)) = ranges {
        //     let ranges = parsed_ranges.validate(meta.len()).unwrap();
        //     if ranges.len() != 1 {
        //         panic!("Multiple ranges not implemented right now")
        //     }

        //     if let Some(range) = ranges.first() {
        //         file.seek(SeekFrom::Start(*range.start())).await.unwrap();
        //     }
        // }

        let body = StreamBody::new(ReaderStream::new(file));

        // get the file type as a string

        print!("file type: {}", file_type);

        (
            StatusCode::OK,
            [
                (
                    header::CONTENT_TYPE,
                    (String::from("video/") + file_type).to_string(),
                ),
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (header::CONTENT_LENGTH, meta.len().to_string()),
            ],
            body,
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            [
                (
                    header::CONTENT_TYPE,
                    "text/plain; charset=utf-8".to_string(),
                ),
                (header::CONTENT_LENGTH, "404 Not Found".len().to_string()),
                (header::ACCESS_CONTROL_MAX_AGE, ("8000").to_string()),
            ],
            "404 Not Found",
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/:id", get(get_file));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
