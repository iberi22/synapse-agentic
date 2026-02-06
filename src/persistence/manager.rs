//! Database Manager - Unified access to all database backends.

use std::sync::Arc;
use anyhow::{Result, bail};
use tracing::info;

use super::adapter::{DatabaseAdapter, GraphAdapter, VectorAdapter};
use super::config::{DatabaseConfig, PrimaryDbConfig, VectorDbConfig};

/// The DatabaseManager provides unified access to all configured databases.
pub struct DatabaseManager {
    primary: Arc<dyn DatabaseAdapter>,
    graph: Option<Arc<dyn GraphAdapter>>,
    vector: Option<Arc<dyn VectorAdapter>>,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Creates a new DatabaseManager from configuration.
    pub async fn from_config(config: DatabaseConfig) -> Result<Self> {
        info!("Initializing database manager");

        let (primary, graph, surreal_as_vector): (
            Arc<dyn DatabaseAdapter>,
            Option<Arc<dyn GraphAdapter>>,
            Option<Arc<dyn VectorAdapter>>
        ) = match &config.primary {
                PrimaryDbConfig::SurrealMemory => {
                    #[cfg(feature = "db-surreal")]
                    {
                        let adapter = super::surreal::SurrealAdapter::memory().await?;
                        let arc = Arc::new(adapter);
                        // Explicitly cast/coerce to trait objects
                        (
                            arc.clone() as Arc<dyn DatabaseAdapter>,
                            Some(arc.clone() as Arc<dyn GraphAdapter>),
                            Some(arc as Arc<dyn VectorAdapter>)
                        )
                    }
                    #[cfg(not(feature = "db-surreal"))]
                    bail!("SurrealDB support not enabled. Add 'db-surreal' feature.")
                }
                PrimaryDbConfig::SurrealFile { path } => {
                    #[cfg(feature = "db-surreal")]
                    {
                        let adapter = super::surreal::SurrealAdapter::file(path).await?;
                        let arc = Arc::new(adapter);
                        (
                            arc.clone() as Arc<dyn DatabaseAdapter>,
                            Some(arc.clone() as Arc<dyn GraphAdapter>),
                            Some(arc as Arc<dyn VectorAdapter>)
                        )
                    }
                    #[cfg(not(feature = "db-surreal"))]
                    {
                        let _ = path;
                        bail!("SurrealDB support not enabled. Add 'db-surreal' feature.")
                    }
                }
                PrimaryDbConfig::SurrealRemote { url, namespace, database, username, password } => {
                    #[cfg(feature = "db-surreal")]
                    {
                        let adapter = super::surreal::SurrealAdapter::remote(
                            url,
                            namespace,
                            database,
                            username.as_deref(),
                            password.as_deref(),
                        ).await?;
                        let arc = Arc::new(adapter);
                        (
                            arc.clone() as Arc<dyn DatabaseAdapter>,
                            Some(arc.clone() as Arc<dyn GraphAdapter>),
                            Some(arc as Arc<dyn VectorAdapter>)
                        )
                    }
                    #[cfg(not(feature = "db-surreal"))]
                    {
                        let _ = (url, namespace, database, username, password);
                        bail!("SurrealDB support not enabled. Add 'db-surreal' feature.")
                    }
                }
                PrimaryDbConfig::Postgres { url, ssl_mode: _ } => {
                    #[cfg(feature = "db-postgres")]
                    {
                        let adapter = super::postgres::PostgresAdapter::connect(url, &config.pool).await?;
                        (Arc::new(adapter) as Arc<dyn DatabaseAdapter>, None, None)
                    }
                    #[cfg(not(feature = "db-postgres"))]
                    {
                        let _ = url;
                        bail!("PostgreSQL support not enabled. Add 'db-postgres' feature.")
                    }
                }
            };

        let vector: Option<Arc<dyn VectorAdapter>> = match &config.vector {
            Some(VectorDbConfig::SurrealNative) => {
                // Already captured above if using SurrealDB
                surreal_as_vector
            }
            Some(VectorDbConfig::PgVector { url, dimensions, index_type: _ }) => {
                #[cfg(feature = "db-pgvector")]
                {
                    let adapter = super::pgvector::PgVectorAdapter::connect(url, *dimensions).await?;
                    Some(Arc::new(adapter))
                }
                #[cfg(not(feature = "db-pgvector"))]
                {
                    let _ = (url, dimensions);
                    bail!("pgvector support not enabled. Add 'db-pgvector' feature.")
                }
            }
            Some(VectorDbConfig::Qdrant { .. }) => {
                bail!("Qdrant support not yet implemented")
            }
            None => surreal_as_vector, // Use SurrealDB's native vector if available
        };

        info!(
            primary = primary.name(),
            has_graph = graph.is_some(),
            has_vector = vector.is_some(),
            "Database manager initialized"
        );

        Ok(Self { primary, graph, vector, config })
    }

    /// Returns the primary database adapter.
    pub fn primary(&self) -> &dyn DatabaseAdapter {
        self.primary.as_ref()
    }

    /// Returns the graph adapter if available.
    pub fn graph(&self) -> Option<&dyn GraphAdapter> {
        self.graph.as_ref().map(|a| a.as_ref())
    }

    /// Returns the vector adapter if available.
    pub fn vector(&self) -> Option<&dyn VectorAdapter> {
        self.vector.as_ref().map(|a| a.as_ref())
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Checks health of all configured databases.
    pub async fn health_check(&self) -> Result<DatabaseHealth> {
        let primary_ok = self.primary.health_check().await.unwrap_or(false);

        let graph_ok = if let Some(ref g) = self.graph {
            g.health_check().await.unwrap_or(false)
        } else {
            true
        };

        let vector_ok = if let Some(ref v) = self.vector {
            v.health_check().await.unwrap_or(false)
        } else {
            true
        };

        Ok(DatabaseHealth {
            primary: primary_ok,
            graph: self.graph.is_some() && graph_ok,
            vector: self.vector.is_some() && vector_ok,
            all_healthy: primary_ok && graph_ok && vector_ok,
        })
    }
}

/// Health status of all database connections.
#[derive(Debug, Clone)]
pub struct DatabaseHealth {
    /// Primary database is healthy.
    pub primary: bool,
    /// Graph database is healthy (if configured).
    pub graph: bool,
    /// Vector database is healthy (if configured).
    pub vector: bool,
    /// All configured databases are healthy.
    pub all_healthy: bool,
}
