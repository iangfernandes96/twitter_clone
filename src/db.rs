use scylla::{Session, SessionBuilder};
use std::error::Error;

pub async fn create_session() -> Result<Session, Box<dyn Error>> {
    let session = SessionBuilder::new()
        .known_node("127.0.0.1:9042")
        .build()
        .await?;

    Ok(session)
}
