//! Database Adapter Trait - Core abstraction for all database backends.
//!
//! This module uses a two-layer approach:
//! - `DatabaseAdapter` (dyn-compatible) - for runtime polymorphism
//! - `TypedDatabaseOps` (extension trait) - for type-safe operations

use anyhow::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Unique identifier for entities across all database backends.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
pub struct EntityId(pub String);

impl EntityId {
    /// Creates a new entity ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for EntityId {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A trait that all entities stored in databases must implement.
pub trait Entity: Serialize + DeserializeOwned + Send + Sync + Debug {
    /// The table/collection name where this entity is stored.
    fn table_name() -> &'static str;

    /// Gets the unique identifier of this entity.
    fn id(&self) -> Option<&EntityId>;

    /// Sets the unique identifier of this entity.
    fn set_id(&mut self, id: EntityId);
}

/// Generic result from database queries.
#[derive(Debug)]
pub struct QueryResult {
    /// The items returned by the query as JSON.
    pub items: Vec<serde_json::Value>,
    /// Total count (for pagination).
    pub total: Option<usize>,
    /// Cursor for next page (if applicable).
    pub cursor: Option<String>,
}

impl QueryResult {
    /// Creates an empty result.
    pub fn empty() -> Self {
        Self {
            items: vec![],
            total: Some(0),
            cursor: None,
        }
    }

    /// Creates a result with a single item.
    pub fn single(item: serde_json::Value) -> Self {
        Self {
            items: vec![item],
            total: Some(1),
            cursor: None,
        }
    }

    /// Creates a result with multiple items.
    pub fn many(items: Vec<serde_json::Value>) -> Self {
        let total = items.len();
        Self {
            items,
            total: Some(total),
            cursor: None,
        }
    }

    /// Deserializes items into a typed vector.
    pub fn into_typed<T: DeserializeOwned>(self) -> Result<Vec<T>> {
        self.items
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(Into::into))
            .collect()
    }
}

/// Pagination parameters for queries.
#[derive(Debug, Clone, Default)]
pub struct Pagination {
    /// Maximum number of items to return.
    pub limit: Option<usize>,
    /// Number of items to skip.
    pub offset: Option<usize>,
    /// Cursor for cursor-based pagination.
    pub cursor: Option<String>,
}

impl Pagination {
    /// Creates new pagination parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the limit.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the offset.
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Filter conditions for queries.
#[derive(Debug, Clone)]
pub enum Filter {
    /// field == value
    Eq(String, serde_json::Value),
    /// field != value
    Ne(String, serde_json::Value),
    /// field > value
    Gt(String, serde_json::Value),
    /// field >= value
    Gte(String, serde_json::Value),
    /// field < value
    Lt(String, serde_json::Value),
    /// field <= value
    Lte(String, serde_json::Value),
    /// field LIKE pattern
    Like(String, String),
    /// field IN [values]
    In(String, Vec<serde_json::Value>),
    /// AND conditions
    And(Vec<Filter>),
    /// OR conditions
    Or(Vec<Filter>),
}

impl Filter {
    /// Creates an equality filter.
    pub fn eq(field: impl Into<String>, value: impl Serialize) -> Self {
        Self::Eq(field.into(), serde_json::to_value(value).unwrap())
    }

    /// Creates a not-equal filter.
    pub fn ne(field: impl Into<String>, value: impl Serialize) -> Self {
        Self::Ne(field.into(), serde_json::to_value(value).unwrap())
    }

    /// Creates a greater-than filter.
    pub fn gt(field: impl Into<String>, value: impl Serialize) -> Self {
        Self::Gt(field.into(), serde_json::to_value(value).unwrap())
    }

    /// Creates a LIKE pattern filter.
    pub fn like(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self::Like(field.into(), pattern.into())
    }

    /// Creates an AND filter.
    pub fn and(filters: Vec<Filter>) -> Self {
        Self::And(filters)
    }

    /// Creates an OR filter.
    pub fn or(filters: Vec<Filter>) -> Self {
        Self::Or(filters)
    }
}

/// Sort order for queries.
#[derive(Debug, Clone)]
pub struct Sort {
    /// Field to sort by.
    pub field: String,
    /// Whether to sort in descending order.
    pub descending: bool,
}

impl Sort {
    /// Creates an ascending sort.
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            descending: false,
        }
    }

    /// Creates a descending sort.
    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            descending: true,
        }
    }
}

/// The core trait that all database adapters must implement.
///
/// This trait is dyn-compatible, using JSON for entity data.
/// For type-safe operations, use the `TypedDatabaseOps` extension trait.
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Returns the name of this adapter (e.g., "surreal", "postgres").
    fn name(&self) -> &str;

    /// Checks if the database connection is healthy.
    async fn health_check(&self) -> Result<bool>;

    /// Creates a new entity in the database.
    async fn create_raw(&self, table: &str, entity: serde_json::Value) -> Result<EntityId>;

    /// Creates multiple entities in a single operation.
    async fn create_many_raw(
        &self,
        table: &str,
        entities: Vec<serde_json::Value>,
    ) -> Result<Vec<EntityId>>;

    /// Retrieves an entity by its ID.
    async fn get_raw(&self, table: &str, id: &EntityId) -> Result<Option<serde_json::Value>>;

    /// Retrieves multiple entities by their IDs.
    async fn get_many_raw(&self, table: &str, ids: &[EntityId]) -> Result<Vec<serde_json::Value>>;

    /// Updates an existing entity.
    async fn update_raw(&self, table: &str, id: &EntityId, entity: serde_json::Value)
        -> Result<()>;

    /// Partially updates an entity with only the provided fields.
    async fn patch_raw(&self, table: &str, id: &EntityId, patch: serde_json::Value) -> Result<()>;

    /// Deletes an entity by its ID.
    async fn delete_raw(&self, table: &str, id: &EntityId) -> Result<bool>;

    /// Queries entities with filters, sorting, and pagination.
    async fn query_raw(
        &self,
        table: &str,
        filter: Option<Filter>,
        sort: Option<Vec<Sort>>,
        pagination: Option<Pagination>,
    ) -> Result<QueryResult>;

    /// Counts entities matching the filter.
    async fn count_raw(&self, table: &str, filter: Option<Filter>) -> Result<usize>;

    /// Checks if an entity with the given ID exists.
    async fn exists_raw(&self, table: &str, id: &EntityId) -> Result<bool>;

    /// Executes a raw query (adapter-specific syntax).
    async fn raw_query(&self, query: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

/// Extension trait providing type-safe database operations.
///
/// This trait is implemented automatically for any `DatabaseAdapter`.
#[async_trait]
pub trait TypedDatabaseOps: DatabaseAdapter {
    /// Creates a new typed entity.
    async fn create<E: Entity>(&self, entity: &E) -> Result<EntityId> {
        let json = serde_json::to_value(entity)?;
        self.create_raw(E::table_name(), json).await
    }

    /// Creates multiple typed entities.
    async fn create_many<E: Entity>(&self, entities: &[E]) -> Result<Vec<EntityId>> {
        let jsons: Vec<serde_json::Value> = entities
            .iter()
            .map(|e| serde_json::to_value(e))
            .collect::<Result<_, _>>()?;
        self.create_many_raw(E::table_name(), jsons).await
    }

    /// Gets a typed entity by ID.
    async fn get<E: Entity>(&self, id: &EntityId) -> Result<Option<E>> {
        match self.get_raw(E::table_name(), id).await? {
            Some(json) => Ok(Some(serde_json::from_value(json)?)),
            None => Ok(None),
        }
    }

    /// Gets multiple typed entities by IDs.
    async fn get_many<E: Entity>(&self, ids: &[EntityId]) -> Result<Vec<E>> {
        let jsons = self.get_many_raw(E::table_name(), ids).await?;
        jsons
            .into_iter()
            .map(|j| serde_json::from_value(j).map_err(Into::into))
            .collect()
    }

    /// Updates a typed entity.
    async fn update<E: Entity>(&self, id: &EntityId, entity: &E) -> Result<()> {
        let json = serde_json::to_value(entity)?;
        self.update_raw(E::table_name(), id, json).await
    }

    /// Deletes a typed entity.
    async fn delete<E: Entity>(&self, id: &EntityId) -> Result<bool> {
        self.delete_raw(E::table_name(), id).await
    }

    /// Queries typed entities.
    async fn query<E: Entity>(
        &self,
        filter: Option<Filter>,
        sort: Option<Vec<Sort>>,
        pagination: Option<Pagination>,
    ) -> Result<Vec<E>> {
        let result = self
            .query_raw(E::table_name(), filter, sort, pagination)
            .await?;
        result.into_typed()
    }

    /// Counts typed entities.
    async fn count<E: Entity>(&self, filter: Option<Filter>) -> Result<usize> {
        self.count_raw(E::table_name(), filter).await
    }

    /// Checks if a typed entity exists.
    async fn exists<E: Entity>(&self, id: &EntityId) -> Result<bool> {
        self.exists_raw(E::table_name(), id).await
    }
}

// Blanket implementation for all DatabaseAdapters
impl<T: DatabaseAdapter + ?Sized> TypedDatabaseOps for T {}

/// Extension trait for graph-capable databases (like SurrealDB).
#[async_trait]
pub trait GraphAdapter: DatabaseAdapter {
    /// Creates a relation between two entities.
    async fn relate(
        &self,
        from: &EntityId,
        relation: &str,
        to: &EntityId,
        data: Option<serde_json::Value>,
    ) -> Result<EntityId>;

    /// Queries relations (graph traversal).
    async fn traverse(
        &self,
        start: &EntityId,
        relation: &str,
        depth: Option<usize>,
    ) -> Result<serde_json::Value>;
}

/// Extension trait for vector-capable databases (like pgvector).
#[async_trait]
pub trait VectorAdapter: DatabaseAdapter {
    /// Stores a vector embedding.
    async fn store_embedding(
        &self,
        id: &EntityId,
        embedding: &[f32],
        metadata: Option<serde_json::Value>,
    ) -> Result<()>;

    /// Searches for similar vectors.
    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
        filter: Option<Filter>,
    ) -> Result<Vec<(EntityId, f32, serde_json::Value)>>;

    /// Deletes an embedding.
    async fn delete_embedding(&self, id: &EntityId) -> Result<bool>;
}
