//! # Persistence Layer - Multi-Database Abstraction
//!
//! Este módulo proporciona una capa de abstracción para conectar múltiples
//! bases de datos de forma transparente.
//!
//! ## Databases Soportadas
//!
//! | Database    | Feature Flag     | Uso Recomendado                    |
//! |-------------|------------------|-----------------------------------|
//! | SurrealDB   | `db-surreal`     | Grafos, documentos, multi-modelo  |
//! | PostgreSQL  | `db-postgres`    | SQL tradicional, transacciones    |
//! | pgvector    | `db-pgvector`    | Embeddings, búsqueda semántica    |
//!
//! ## Arquitectura
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Application Layer                     │
//! │              (Agents, Tools, Workflows)                  │
//! └────────────────────────┬────────────────────────────────┘
//!                          │
//! ┌────────────────────────▼────────────────────────────────┐
//! │                   DatabaseManager                        │
//! │    ┌──────────┐ ┌──────────┐ ┌──────────┐               │
//! │    │ surreal  │ │ postgres │ │ pgvector │               │
//! │    └────┬─────┘ └────┬─────┘ └────┬─────┘               │
//! └─────────┼────────────┼────────────┼─────────────────────┘
//!           │            │            │
//! ┌─────────▼──────┐ ┌───▼────┐ ┌────▼─────┐
//! │   SurrealDB    │ │ PgPool │ │ pgvector │
//! │  (embedded/    │ │        │ │          │
//! │   cloud)       │ │        │ │          │
//! └────────────────┘ └────────┘ └──────────┘
//! ```
//!
//! ## Ejemplo de Uso
//!
//! ```rust,no_run
//! use synapse_agentic::persistence::{DatabaseManager, DatabaseConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize with default config (SurrealDB in-memory)
//!     let config = DatabaseConfig::default();
//!     let manager = DatabaseManager::from_config(config).await?;
//!
//!     // Use the manager to access data
//!     // ...
//!
//!     Ok(())
//! }
//! ```

pub mod adapter;
pub mod manager;
pub mod config;

#[cfg(feature = "db-surreal")]
pub mod surreal;

#[cfg(feature = "db-postgres")]
pub mod postgres;

#[cfg(feature = "db-pgvector")]
pub mod pgvector;

// Re-exports
pub use adapter::{DatabaseAdapter, TypedDatabaseOps, GraphAdapter, VectorAdapter, QueryResult, Entity, EntityId, Filter, Sort, Pagination};
pub use manager::{DatabaseManager, DatabaseHealth};
pub use config::{DatabaseConfig, PrimaryDbConfig, VectorDbConfig, PoolConfig};

#[cfg(feature = "db-surreal")]
pub use surreal::SurrealAdapter;

#[cfg(feature = "db-postgres")]
pub use postgres::PostgresAdapter;

#[cfg(feature = "db-pgvector")]
pub use pgvector::PgVectorAdapter;
