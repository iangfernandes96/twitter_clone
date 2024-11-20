use bb8::{ManageConnection, Pool};
use scylla::{Session, SessionBuilder};
use std::error::Error;
use std::sync::Arc;
type DbPool = Pool<ScyllaConnectionManager>;
use async_trait::async_trait;

// pub async fn create_session() -> Result<Session, Box<dyn Error>> {
//     let session = SessionBuilder::new()
//         .known_node("127.0.0.1:9042")
//         .build()
//         .await?;

//     Ok(session)
// }

pub struct ScyllaConnectionManager {
    connection_string: String,
}

impl ScyllaConnectionManager {
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
        }
    }
}

#[async_trait]
impl ManageConnection for ScyllaConnectionManager {
    type Connection = Arc<Session>;
    type Error = scylla::transport::errors::NewSessionError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let session = SessionBuilder::new()
            .known_node(&self.connection_string)
            .build()
            .await?;
        Ok(Arc::new(session))
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.query("SELECT release_version FROM system.local", &[])
            .await
            .map(|_| ())
            .map_err(|e| Self::Error::from(e))
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}

pub async fn create_connection_pool() -> Result<DbPool, Box<dyn Error>> {
    let manager = ScyllaConnectionManager::new("127.0.0.1:9042");
    let pool = Pool::builder()
        .max_size(30) // Number of connections
        .build(manager)
        .await
        .expect("Failed to create pool");
    Ok(pool)
}
