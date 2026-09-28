//! 图片提取与远程图片 SSRF 防护的回归测试。

use super::*;

#[tokio::test]
async fn build_kiro_payload_extracts_base64_images() {
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!([
                {
                    "type": "text",
                    "text": "看图回答"
                },
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "aGVsbG8="
                    }
                }
            ])),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");
    let current = &payload
        .conversation_state
        .current_message
        .user_input_message;

    assert_eq!(current.images.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        current
            .images
            .as_ref()
            .and_then(|images| images.first())
            .map(|image| image.format.as_str()),
        Some("png")
    );
}

#[tokio::test]
async fn build_kiro_payload_rejects_private_remote_images() {
    let expected_bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    listener
        .set_nonblocking(true)
        .expect("listener should set nonblocking");
    let address = format!(
        "http://{}",
        listener.local_addr().expect("local addr should resolve")
    );

    let handle = thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buffer = [0u8; 1024];
                    let _ = stream.read(&mut buffer);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        expected_bytes.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("headers should write");
                    stream
                        .write_all(&expected_bytes)
                        .expect("body should write");
                    return true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return false;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(_) => return false,
            }
        }
    });

    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!([
                {
                    "type": "text",
                    "text": "看图回答"
                },
                {
                    "type": "input_image",
                    "image_url": format!("{address}/sample.png")
                }
            ])),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");
    assert!(
        !handle.join().expect("server thread should finish"),
        "client should not fetch private image"
    );
    let current = &payload
        .conversation_state
        .current_message
        .user_input_message;

    assert!(current.images.as_ref().map(Vec::is_empty).unwrap_or(true));
}

#[tokio::test]
async fn build_kiro_payload_rejects_oversized_data_url_images() {
    let oversized = STANDARD.encode(vec![0u8; 6 * 1024 * 1024]);
    let request = NormalizedRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        messages: vec![NormalizedMessage {
            role: "user".to_string(),
            content: Some(json!([
                {
                    "type": "text",
                    "text": "看图回答"
                },
                {
                    "type": "input_image",
                    "image_url": format!("data:image/png;base64,{oversized}")
                }
            ])),
            tool_calls: None,
            tool_call_id: None,
            metadata: None,
        }],
        stream: false,
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        previous_response_id: None,
        thinking: None,
        include_usage: false,
        tool_name_map: Default::default(),
        server_tool_names: Vec::new(),
    };

    let payload = build_kiro_payload(&Client::new(), &request, None, None)
        .await
        .expect("payload should build");
    let current = &payload
        .conversation_state
        .current_message
        .user_input_message;

    assert!(current.images.as_ref().map(Vec::is_empty).unwrap_or(true));
}

#[tokio::test]
async fn extract_images_works_with_preserved_image_array() {
    // 测试：extract_images 能从保留的数组中提取图片
    let content = json!([
        {
            "type": "text",
            "text": "这是什么图片？"
        },
        {
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
            }
        }
    ]);

    let client = Client::new();
    let images = extract_images(&client, Some(&content)).await;

    // 验证成功提取了图片
    assert_eq!(images.len(), 1, "should extract 1 image");
    assert_eq!(images[0].format, "png", "image format should be png");

    // 验证图片数据
    match &images[0].source {
        ImageSource::Bytes { bytes } => {
            assert!(!bytes.is_empty(), "image bytes should not be empty");
            assert_eq!(
                bytes,
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
                "image bytes should match"
            );
        }
        ImageSource::Other { .. } => {
            panic!("expected ImageSource::Bytes, got ImageSource::Other");
        }
    }
}
