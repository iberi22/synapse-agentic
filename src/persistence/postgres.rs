//! PostgreSQL Adapter - Traditional SQL database support.

use async_trait::async_trait;
use anyhow::Result;
use tracing::{debug, info};

use super::adapter::{
    DatabaseAdapter, EntityId, Filter, Pagination, QueryResult, Sort,
};
use super::config::PoolConfig;

/// PostgreSQL adapter (stub - requires sqlx dependency).
pub struct PostgresAdapter {
    #[allow(dead_code)]
    connection_url: String,
    #[allow(dead_code)]
    pool_config: PoolConfig,
}

impl PostgresAdapter {
    /// Connects to a PostgreSQL database.
    pub async fn connect(url: &str, pool_config: &PoolConfig) -> Result<Self> {
        info!(url = %url.split('@').last().unwrap_or("hidden"), "Connecting to PostgreSQL");
        Ok(Self {
            connection_url: url.to_string(),
            pool_config: pool_config.clone(),
        })
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    fn name(&self) -> &str { "postgres" }

    async fn health_check(&self) -> Result<bool> {
        debug!("PostgreSQL health check (stub)");
        Ok(true)
    }

    async fn create_raw(&self, table: &str, _entity: serde_json::Value) -> Result<EntityId> {
        debug!(table, "Creating entity in PostgreSQL (stub)");
        Ok(EntityId::new(uuid::Uuid::new_v4().to_string()))
    }

    async fn create_many_raw(&self, table: &str, entities: Vec<serde_json::Value>) -> Result<Vec<EntityId>> {
        let mut ids = Vec::new();
        for entity in entities {
            ids.push(self.create_raw(table, entity).await?);
        }
        Ok(ids)
    }

    async fn get_raw(&self, _table: &str, _id: &EntityId) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    async fn get_many_raw(&self, _table: &str, _ids: &[EntityId]) -> Result<Vec<serde_json::Value>> {
        Ok(vec![])
    }

    async fn update_raw(&self, _table: &str, _id: &EntityId, _entity: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn patch_raw(&self, _table: &str, _id: &EntityId, _patch: serde_json::Value) -> Result<()> {
        Ok(())
    }

    async fn delete_raw(&self, _table: &str, _id: &EntityId) -> Result<bool> {
        Ok(true)
    }

    async fn query_raw(
        &self,
        _table: &str,
        _filter: Option<Filter>,
        _sort: Option<Vec<Sort>>,
        _pagination: Option<Pagination>,
    ) -> Result<QueryResult> {
        Ok(QueryResult::empty())
    }

    async fn count_raw(&self, _table: &str, _filter: Option<Filter>) -> Result<usize> {
        Ok(0)
    }

    async fn exists_raw(&self, _table: &str, _id: &EntityId) -> Result<bool> {
        Ok(false)
    }

    async fn raw_query(&self, _query: &str, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
}
