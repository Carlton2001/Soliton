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
