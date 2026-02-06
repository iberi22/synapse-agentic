//! Database configuration for all supported backends.

use serde::{Deserialize, Serialize};

/// Configuration for connecting to databases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Primary database configuration.
    pub primary: PrimaryDbConfig,
    /// Optional vector database configuration.
    pub vector: Option<VectorDbConfig>,
    /// Connection pool settings.
    pub pool: PoolConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            primary: PrimaryDbConfig::SurrealMemory,
            vector: None,
            pool: PoolConfig::default(),
        }
    }
}

/// Primary database backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PrimaryDbConfig {
    /// SurrealDB in-memory (development/testing).
    SurrealMemory,

    /// SurrealDB with file-based storage.
    SurrealFile {
        /// File path for storage.
        path: String,
    },

    /// SurrealDB cloud/self-hosted.
    SurrealRemote {
        /// Connection URL.
        url: String,
        /// SurrealDB namespace.
        namespace: String,
        /// SurrealDB database name.
        database: String,
        /// Authentication username.
        username: Option<String>,
        /// Authentication password.
        password: Option<String>,
    },

    /// PostgreSQL database.
    Postgres {
        /// PostgreSQL connection URL.
        url: String,
        /// SSL mode (disable, prefer, require)
        ssl_mode: Option<String>,
    },
}

/// Vector database configuration for embeddings/semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VectorDbConfig {
    /// Use SurrealDB's native vector capabilities.
    SurrealNative,

    /// PostgreSQL with pgvector extension.
    PgVector {
        /// PostgreSQL connection URL.
        url: String,
        /// Vector dimensions (e.g., 1536 for OpenAI ada-002).
        dimensions: usize,
        /// Index type (ivfflat, hnsw).
        index_type: Option<String>,
    },

    /// Qdrant vector database.
    Qdrant {
        /// Qdrant server URL.
        url: String,
        /// API key for authentication.
        api_key: Option<String>,
        /// Collection name.
        collection: String,
    },
}

/// Connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections.
    pub max_connections: u32,
    /// Minimum number of idle connections.
    pub min_connections: u32,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Idle connection timeout in seconds.
    pub idle_timeout_secs: u64,
    /// Maximum lifetime of a connection in seconds.
    pub max_lifetime_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        }
    }
}

impl DatabaseConfig {
    /// Creates a new configuration for in-memory SurrealDB (development).
    pub fn surreal_memory() -> Self {
        Self {
            primary: PrimaryDbConfig::SurrealMemory,
            vector: Some(VectorDbConfig::SurrealNative),
            pool: PoolConfig::default(),
        }
    }

    /// Creates a new configuration for file-based SurrealDB.
    pub fn surreal_file(path: impl Into<String>) -> Self {
        Self {
            primary: PrimaryDbConfig::SurrealFile { path: path.into() },
            vector: Some(VectorDbConfig::SurrealNative),
            pool: PoolConfig::default(),
        }
    }

    /// Creates a new configuration for remote SurrealDB.
    pub fn surreal_remote(
        url: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            primary: PrimaryDbConfig::SurrealRemote {
                url: url.into(),
                namespace: namespace.into(),
                database: database.into(),
                username: None,
                password: None,
            },
            vector: Some(VectorDbConfig::SurrealNative),
            pool: PoolConfig::default(),
        }
    }

    /// Creates a new configuration for PostgreSQL.
    pub fn postgres(url: impl Into<String>) -> Self {
        Self {
            primary: PrimaryDbConfig::Postgres {
                url: url.into(),
                ssl_mode: Some("prefer".into()),
            },
            vector: None,
            pool: PoolConfig::default(),
        }
    }

    /// Creates a new configuration for PostgreSQL with pgvector.
    pub fn postgres_with_vector(url: impl Into<String>, vector_url: impl Into<String>) -> Self {
        Self {
            primary: PrimaryDbConfig::Postgres {
                url: url.into(),
                ssl_mode: Some("prefer".into()),
            },
            vector: Some(VectorDbConfig::PgVector {
                url: vector_url.into(),
                dimensions: 1536, // OpenAI ada-002 default
                index_type: Some("hnsw".into()),
            }),
            pool: PoolConfig::default(),
        }
    }

    /// Adds credentials for SurrealDB remote.
    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        if let PrimaryDbConfig::SurrealRemote {
            username: ref mut u,
            password: ref mut p,
            ..
        } = self.primary {
            *u = Some(username.into());
            *p = Some(password.into());
        }
        self
    }

    /// Sets pool configuration.
    pub fn with_pool(mut self, pool: PoolConfig) -> Self {
        self.pool = pool;
        self
    }
}
