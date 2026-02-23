//! SurrealDB Adapter - Multi-model database with graph capabilities.

use anyhow::{Context, Result};
use async_trait::async_trait;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tracing::{debug, info};

use super::adapter::{
    DatabaseAdapter, EntityId, Filter, GraphAdapter, Pagination, QueryResult, Sort, VectorAdapter,
};

/// SurrealDB adapter implementing all database traits.
pub struct SurrealAdapter {
    db: Surreal<Any>,
    #[allow(dead_code)]
    namespace: String,
    #[allow(dead_code)]
    database: String,
}

impl SurrealAdapter {
    /// Creates an in-memory SurrealDB instance (for development/testing).
    pub async fn memory() -> Result<Self> {
        info!("Connecting to SurrealDB (in-memory)");
        let db = Surreal::<Any>::init();
        db.connect("mem://")
            .await
            .context("Failed to connect to SurrealDB memory")?;
        db.use_ns("synapse").use_db("enterprise").await?;

        Ok(Self {
            db,
            namespace: "synapse".into(),
            database: "enterprise".into(),
        })
    }

    /// Creates a file-based SurrealDB instance.
    pub async fn file(path: &str) -> Result<Self> {
        info!(path, "Connecting to SurrealDB (file)");
        let db = Surreal::<Any>::init();
        db.connect(format!("surrealkv://{}", path))
            .await
            .context("Failed to connect to SurrealDB file")?;
        db.use_ns("synapse").use_db("enterprise").await?;

        Ok(Self {
            db,
            namespace: "synapse".into(),
            database: "enterprise".into(),
        })
    }

    /// Connects to a remote SurrealDB instance.
    pub async fn remote(
        url: &str,
        namespace: &str,
        database: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self> {
        info!(url, namespace, database, "Connecting to SurrealDB (remote)");
        let db = Surreal::<Any>::init();
        db.connect(url)
            .await
            .context("Failed to connect to SurrealDB")?;

        if let (Some(user), Some(pass)) = (username, password) {
            db.signin(Root {
                username: user.to_string(),
                password: pass.to_string(),
            })
            .await
            .context("Failed to authenticate with SurrealDB")?;
        }

        db.use_ns(namespace).use_db(database).await?;

        Ok(Self {
            db,
            namespace: namespace.into(),
            database: database.into(),
        })
    }

    /// Returns a reference to the underlying Surreal client.
    pub fn client(&self) -> &Surreal<Any> {
        &self.db
    }

    fn build_filter(filter: &Filter) -> String {
        match filter {
            Filter::Eq(field, value) => format!("{} = {}", field, value),
            Filter::Ne(field, value) => format!("{} != {}", field, value),
            Filter::Gt(field, value) => format!("{} > {}", field, value),
            Filter::Gte(field, value) => format!("{} >= {}", field, value),
            Filter::Lt(field, value) => format!("{} < {}", field, value),
            Filter::Lte(field, value) => format!("{} <= {}", field, value),
            Filter::Like(field, pattern) => format!("{} ~ '{}'", field, pattern),
            Filter::In(field, values) => {
                let vals: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                format!("{} IN [{}]", field, vals.join(", "))
            }
            Filter::And(filters) => {
                let parts: Vec<String> = filters.iter().map(Self::build_filter).collect();
                format!("({})", parts.join(" AND "))
            }
            Filter::Or(filters) => {
                let parts: Vec<String> = filters.iter().map(Self::build_filter).collect();
                format!("({})", parts.join(" OR "))
            }
        }
    }
}

#[async_trait]
impl DatabaseAdapter for SurrealAdapter {
    fn name(&self) -> &str {
        "surreal"
    }

    async fn health_check(&self) -> Result<bool> {
        let result: Option<i32> = self.db.query("RETURN 1").await?.take(0)?;
        Ok(result == Some(1))
    }

    async fn create_raw(&self, table: &str, entity: serde_json::Value) -> Result<EntityId> {
        debug!(table, "Creating entity");

        let created: Option<serde_json::Value> = self.db.create(table).content(entity).await?;

        let id = created
            .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(EntityId::new(id))
    }

    async fn create_many_raw(
        &self,
        table: &str,
        entities: Vec<serde_json::Value>,
    ) -> Result<Vec<EntityId>> {
        debug!(table, count = entities.len(), "Creating multiple entities");

        let mut ids = Vec::with_capacity(entities.len());
        for entity in entities {
            let id = self.create_raw(table, entity).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn get_raw(&self, table: &str, id: &EntityId) -> Result<Option<serde_json::Value>> {
        debug!(table, id = %id.as_str(), "Getting entity");
        let result: Option<serde_json::Value> = self.db.select((table, id.as_str())).await?;
        Ok(result)
    }

    async fn get_many_raw(&self, table: &str, ids: &[EntityId]) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(entity) = self.get_raw(table, id).await? {
                results.push(entity);
            }
        }
        Ok(results)
    }

    async fn update_raw(
        &self,
        table: &str,
        id: &EntityId,
        entity: serde_json::Value,
    ) -> Result<()> {
        debug!(table, id = %id.as_str(), "Updating entity");
        let _: Option<serde_json::Value> =
            self.db.update((table, id.as_str())).content(entity).await?;
        Ok(())
    }

    async fn patch_raw(&self, table: &str, id: &EntityId, patch: serde_json::Value) -> Result<()> {
        debug!(table, id = %id.as_str(), "Patching entity");
        let _: Option<serde_json::Value> =
            self.db.update((table, id.as_str())).merge(patch).await?;
        Ok(())
    }

    async fn delete_raw(&self, table: &str, id: &EntityId) -> Result<bool> {
        debug!(table, id = %id.as_str(), "Deleting entity");
        let deleted: Option<serde_json::Value> = self.db.delete((table, id.as_str())).await?;
        Ok(deleted.is_some())
    }

    async fn query_raw(
        &self,
        table: &str,
        filter: Option<Filter>,
        sort: Option<Vec<Sort>>,
        pagination: Option<Pagination>,
    ) -> Result<QueryResult> {
        let mut query = format!("SELECT * FROM {}", table);

        if let Some(ref f) = filter {
            query.push_str(&format!(" WHERE {}", Self::build_filter(f)));
        }

        if let Some(ref sorts) = sort {
            let sort_parts: Vec<String> = sorts
                .iter()
                .map(|s| {
                    if s.descending {
                        format!("{} DESC", s.field)
                    } else {
                        format!("{} ASC", s.field)
                    }
                })
                .collect();
            query.push_str(&format!(" ORDER BY {}", sort_parts.join(", ")));
        }

        if let Some(ref p) = pagination {
            if let Some(limit) = p.limit {
                query.push_str(&format!(" LIMIT {}", limit));
            }
            if let Some(offset) = p.offset {
                query.push_str(&format!(" START {}", offset));
            }
        }

        debug!(query, "Executing query");

        let mut response = self.db.query(&query).await?;
        let items: Vec<serde_json::Value> = response.take(0)?;

        Ok(QueryResult::many(items))
    }

    async fn count_raw(&self, table: &str, filter: Option<Filter>) -> Result<usize> {
        let mut query = format!("SELECT count() FROM {} GROUP ALL", table);

        if let Some(ref f) = filter {
            query = format!(
                "SELECT count() FROM {} WHERE {} GROUP ALL",
                table,
                Self::build_filter(f)
            );
        }

        let mut response = self.db.query(&query).await?;
        let result: Option<serde_json::Value> = response.take(0)?;

        let count = result
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0) as usize;

        Ok(count)
    }

    async fn exists_raw(&self, table: &str, id: &EntityId) -> Result<bool> {
        let entity = self.get_raw(table, id).await?;
        Ok(entity.is_some())
    }

    async fn raw_query(&self, query: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!(query, "Executing raw query");
        let mut response = self.db.query(query).bind(params).await?;
        let result: Option<serde_json::Value> = response.take(0usize)?;
        let json = serde_json::to_value(&result)?;
        Ok(json)
    }
}

#[async_trait]
impl GraphAdapter for SurrealAdapter {
    async fn relate(
        &self,
        from: &EntityId,
        relation: &str,
        to: &EntityId,
        _data: Option<serde_json::Value>,
    ) -> Result<EntityId> {
        let query = format!("RELATE {}->{}->{}", from.as_str(), relation, to.as_str(),);

        debug!(query, "Creating relation");

        let mut response = self.db.query(&query).await?;
        let result: Option<serde_json::Value> = response.take(0)?;

        let id = result
            .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
            .unwrap_or_else(|| format!("{}->{}->{}", from.as_str(), relation, to.as_str()));

        Ok(EntityId::new(id))
    }

    async fn traverse(
        &self,
        start: &EntityId,
        relation: &str,
        depth: Option<usize>,
    ) -> Result<serde_json::Value> {
        let depth_clause = depth.map(|d| format!("..{}", d)).unwrap_or_default();
        let query = format!(
            "SELECT ->{}{} FROM {}",
            relation,
            depth_clause,
            start.as_str()
        );

        debug!(query, "Traversing graph");

        let mut response = self.db.query(&query).await?;
        let result: Option<serde_json::Value> = response.take(0usize)?;
        let json = serde_json::to_value(&result)?;

        Ok(json)
    }
}

#[async_trait]
impl VectorAdapter for SurrealAdapter {
    async fn store_embedding(
        &self,
        id: &EntityId,
        embedding: &[f32],
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let data = serde_json::json!({
            "embedding": embedding,
            "metadata": metadata
        });

        let _: Option<serde_json::Value> = self
            .db
            .update(("embeddings", id.as_str()))
            .content(data)
            .await?;

        Ok(())
    }

    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
        _filter: Option<Filter>,
    ) -> Result<Vec<(EntityId, f32, serde_json::Value)>> {
        let query = format!(
            "SELECT id, metadata, vector::similarity::cosine(embedding, $embedding) AS score \
             FROM embeddings \
             ORDER BY score DESC \
             LIMIT {}",
            limit
        );

        // Clone the embedding to avoid lifetime issues with the query
        let embedding_vec: Vec<f32> = embedding.to_vec();

        let mut response = self
            .db
            .query(&query)
            .bind(("embedding", embedding_vec))
            .await?;

        let results: Vec<serde_json::Value> = response.take(0)?;

        let mapped: Vec<(EntityId, f32, serde_json::Value)> = results
            .into_iter()
            .filter_map(|v| {
                let id = v.get("id")?.as_str()?.to_string();
                let score = v.get("score")?.as_f64()? as f32;
                let metadata = v
                    .get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                Some((EntityId::new(id), score, metadata))
            })
            .collect();

        Ok(mapped)
    }

    async fn delete_embedding(&self, id: &EntityId) -> Result<bool> {
        let deleted: Option<serde_json::Value> =
            self.db.delete(("embeddings", id.as_str())).await?;
        Ok(deleted.is_some())
    }
}
