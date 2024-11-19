use futures::future::try_join_all;
use scylla::{Session, SessionBuilder};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn create_session() -> Result<Session, Box<dyn Error>> {
    let session = SessionBuilder::new()
        .known_node("127.0.0.1:9042")
        .build()
        .await?;

    Ok(session)
}

pub async fn create_connection_pool() -> Result<Arc<Mutex<Vec<Session>>>, Box<dyn Error>> {
    let session_futures = (0..num_cpus::get() * 8)
        .map(|_| async {
            SessionBuilder::new()
                .known_node("127.0.0.1:9042")
                .build()
                .await
        })
        .collect::<Vec<_>>();

    let pool = try_join_all(session_futures).await?;

    Ok(Arc::new(Mutex::new(pool)))
}
