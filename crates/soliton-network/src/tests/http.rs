use super::*;

#[tokio::test]
async fn server_serves_only_granted_media_with_ranges_head_cors_and_revocation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    std::fs::write(&path, b"0123456789").unwrap();
    let cover = dir.path().join("fixture.png");
    std::fs::write(&cover, b"PNG fixture").unwrap();
    let track = Track {
        id: 1,
        source_id: 1,
        path: path.clone(),
        metadata: Default::default(),
        cover: Some(cover),
        available: true,
    };
    let server = Server::start("127.0.0.1".parse().unwrap()).await.unwrap();
    let urls = server.authorize(&track, "audio/wav".into()).await.unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .get(&urls.audio)
        .header(header::RANGE, "bytes=3-6")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 3-6/10");
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert_eq!(response.bytes().await.unwrap(), "3456");
    let response = client.head(&urls.audio).send().await.unwrap();
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "10");
    assert!(response.bytes().await.unwrap().is_empty());
    assert_eq!(
        client
            .get(&urls.audio)
            .header(header::RANGE, "bytes=99-")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::RANGE_NOT_SATISFIABLE
    );
    assert_eq!(
        client
            .request(Method::OPTIONS, &urls.audio)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .get(urls.cover.unwrap())
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        "PNG fixture"
    );
    assert_eq!(
        client
            .get(format!("http://{}/unknown/audio", server.address))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let next = server.authorize(&track, "audio/wav".into()).await.unwrap();
    assert_eq!(
        client.get(&urls.audio).send().await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    server.revoke().await;
    assert_eq!(
        client.get(next.audio).send().await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(std::fs::read(path).unwrap(), b"0123456789");
}

#[tokio::test]
async fn another_peer_cannot_use_even_a_valid_capability_url() {
    use tower::ServiceExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.wav");
    std::fs::write(&path, b"private").unwrap();
    let access = Access {
        receiver: "192.168.1.42".parse().unwrap(),
        streams: Arc::new(Semaphore::new(1)),
        grant: Arc::new(RwLock::new(Some(Grant {
            token: "known-token".into(),
            audio: path,
            cover: None,
            mime: "audio/wav".into(),
            valid: Arc::new(AtomicBool::new(true)),
        }))),
    };
    let app = Router::new()
        .route("/{token}/{kind}", get(serve))
        .with_state(access);
    let mut request = axum::http::Request::builder()
        .uri("/known-token/audio")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "192.168.1.43:1234".parse::<SocketAddr>().unwrap(),
    ));
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

fn local_track(path: PathBuf, cover: Option<PathBuf>) -> Track {
    Track {
        id: 1,
        source_id: 1,
        path,
        metadata: Default::default(),
        cover,
        available: true,
    }
}

#[tokio::test]
async fn only_readable_available_files_are_authorized_and_a_missing_cover_is_left_out() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start("127.0.0.1".parse().unwrap()).await.unwrap();
    let path = dir.path().join("fixture.wav");
    std::fs::write(&path, b"0123456789").unwrap();
    let mut offline = local_track(path.clone(), None);
    offline.available = false;
    assert!(
        server
            .authorize(&offline, "audio/wav".into())
            .await
            .is_err()
    );
    let missing = local_track(dir.path().join("gone.wav"), None);
    assert!(
        server
            .authorize(&missing, "audio/wav".into())
            .await
            .is_err()
    );
    let folder = local_track(dir.path().to_path_buf(), None);
    assert!(server.authorize(&folder, "audio/wav".into()).await.is_err());
    let urls = server
        .authorize(
            &local_track(path, Some(dir.path().join("no-cover.png"))),
            "audio/wav".into(),
        )
        .await
        .unwrap();
    assert!(urls.cover.is_none());
    assert_eq!(urls.mime, "audio/wav");
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let cover = urls.audio.replace("/audio", "/cover");
    assert_eq!(
        client.get(cover).send().await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    // A Range in another unit is ignored: the whole file, as a plain answer.
    let response = client
        .get(&urls.audio)
        .header(header::RANGE, "items=0-1")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::CONTENT_RANGE).is_none());
    assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/wav");
    assert_eq!(response.bytes().await.unwrap(), "0123456789");
}

/// Requests from the granted receiver, answered without a listening socket.
async fn request(access: Access, uri: &str) -> StatusCode {
    use tower::ServiceExt;
    let app = Router::new()
        .route("/{token}/{kind}", get(serve))
        .with_state(access);
    let mut request = axum::http::Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "192.168.1.42:1234".parse::<SocketAddr>().unwrap(),
    ));
    app.oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn unknown_kinds_and_saturated_streams_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    std::fs::write(&path, b"0123456789").unwrap();
    let access = |streams| Access {
        receiver: "192.168.1.42".parse().unwrap(),
        streams: Arc::new(Semaphore::new(streams)),
        grant: Arc::new(RwLock::new(Some(Grant {
            token: "known-token".into(),
            audio: path.clone(),
            cover: None,
            mime: "audio/wav".into(),
            valid: Arc::new(AtomicBool::new(true)),
        }))),
    };
    assert_eq!(
        request(access(1), "/known-token/audio").await,
        StatusCode::OK
    );
    assert_eq!(
        request(access(1), "/known-token/lyrics").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(access(1), "/other-token/audio").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(access(0), "/known-token/audio").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
