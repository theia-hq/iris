use std::time::Duration;

use bifrost::{NoDiscovery, Node};
use bifrost_mem::MemTransport;
use iris::recv::RecvCmd;
use iris::send::SendCmd;

/// Multiple files transfer intact from sender to receiver over the in-memory transport.
#[tokio::test]
async fn transfers_multiple_files() {
    let base = std::env::temp_dir().join(format!("iris-test-{}", std::process::id()));
    let src = base.join("src");
    let out = base.join("out");
    tokio::fs::create_dir_all(&src).await.unwrap();
    tokio::fs::create_dir_all(&out).await.unwrap();

    let alpha = src.join("alpha.txt");
    let beta = src.join("beta.bin");
    tokio::fs::write(&alpha, b"alpha contents").await.unwrap();
    let beta_bytes: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    tokio::fs::write(&beta, &beta_bytes).await.unwrap();

    let receiver = Node::new(MemTransport::bind(), NoDiscovery);
    let receiver_id = receiver.node_id();
    let sender = Node::new(MemTransport::bind(), NoDiscovery);

    tokio::time::timeout(Duration::from_secs(10), async {
        let (sent, received) = tokio::join!(
            SendCmd {
                peer: receiver_id,
                paths: vec![alpha.clone(), beta.clone()]
            }
            .run(&sender),
            RecvCmd { out: out.clone() }.run(&receiver),
        );
        sent.unwrap();
        received.unwrap();
    })
    .await
    .expect("transfer timed out");

    assert_eq!(
        tokio::fs::read(out.join("alpha.txt")).await.unwrap(),
        b"alpha contents"
    );
    assert_eq!(
        tokio::fs::read(out.join("beta.bin")).await.unwrap(),
        beta_bytes
    );

    let _ = tokio::fs::remove_dir_all(&base).await;
}
