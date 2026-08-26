use core::time::Duration;

use bifrost::{NoDiscovery, Node};
use bifrost_mem::MemTransport;
use iris::recv::RecvCmd;
use iris::send::SendCmd;

/// A standalone file and a nested directory transfer intact, tree preserved, over the mem transport.
#[tokio::test]
async fn transfers_files_and_directories() {
    let base = std::env::temp_dir().join(format!("iris-test-{}", std::process::id()));
    let src = base.join("src");
    let nested = src.join("nested");
    let out = base.join("out");
    tokio::fs::create_dir_all(&nested).await.unwrap();
    tokio::fs::create_dir_all(&out).await.unwrap();

    let solo = base.join("solo.txt");
    tokio::fs::write(&solo, b"solo file").await.unwrap();
    tokio::fs::write(src.join("alpha.txt"), b"alpha contents")
        .await
        .unwrap();
    let beta_bytes: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    tokio::fs::write(src.join("beta.bin"), &beta_bytes)
        .await
        .unwrap();
    tokio::fs::write(nested.join("gamma.txt"), b"deep file")
        .await
        .unwrap();

    let receiver = Node::new(MemTransport::bind(), NoDiscovery);
    let receiver_id = receiver.node_id();
    let sender = Node::new(MemTransport::bind(), NoDiscovery);

    tokio::time::timeout(Duration::from_secs(10), async {
        let (sent, received) = tokio::join!(
            SendCmd {
                peer: receiver_id,
                paths: vec![solo.clone(), src.clone()]
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
        tokio::fs::read(out.join("solo.txt")).await.unwrap(),
        b"solo file"
    );
    assert_eq!(
        tokio::fs::read(out.join("src/alpha.txt")).await.unwrap(),
        b"alpha contents"
    );
    assert_eq!(
        tokio::fs::read(out.join("src/beta.bin")).await.unwrap(),
        beta_bytes
    );
    assert_eq!(
        tokio::fs::read(out.join("src/nested/gamma.txt"))
            .await
            .unwrap(),
        b"deep file"
    );

    let _ = tokio::fs::remove_dir_all(&base).await;
}

/// A missing path is skipped and reported; the good file still transfers and send reports the failure.
#[tokio::test]
async fn skips_missing_paths_and_continues() {
    let base = std::env::temp_dir().join(format!("iris-skip-test-{}", std::process::id()));
    let out = base.join("out");
    tokio::fs::create_dir_all(&out).await.unwrap();
    let good = base.join("good.txt");
    tokio::fs::write(&good, b"i made it").await.unwrap();
    let missing = base.join("nope.txt");

    let receiver = Node::new(MemTransport::bind(), NoDiscovery);
    let receiver_id = receiver.node_id();
    let sender = Node::new(MemTransport::bind(), NoDiscovery);

    let (sent, received) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            SendCmd {
                peer: receiver_id,
                paths: vec![good.clone(), missing]
            }
            .run(&sender),
            RecvCmd { out: out.clone() }.run(&receiver),
        )
    })
    .await
    .expect("transfer timed out");

    assert!(sent.is_err(), "send reports the skipped file");
    received.unwrap();
    assert_eq!(
        tokio::fs::read(out.join("good.txt")).await.unwrap(),
        b"i made it"
    );

    let _ = tokio::fs::remove_dir_all(&base).await;
}
