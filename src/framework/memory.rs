//! Memory abstractions for agent state persistence.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents a piece of information stored in memory.
///
/// Memory fragments are the atomic units of agent memory,
/// containing content along with metadata for retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    /// Unique identifier for this fragment.
    pub id: String,

    /// The actual content stored.
    pub content: String,

    /// Context category (e.g., "conversation", "task", "knowledge").
    pub context: String,

    /// Unix timestamp when this was created.
    pub timestamp: i64,

    /// Optional metadata as JSON.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl MemoryFragment {
    /// Creates a new memory fragment.
    pub fn new(content: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            context: context.into(),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Adds metadata to the fragment.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// The `MemoryStore` trait abstracts the storage mechanism.
///
/// Implement this trait to provide different backends:
/// - In-memory (fast, ephemeral)
/// - Database (persistent)
/// - Vector store (semantic search)
///
/// # Example
///
/// ```rust,no_run
/// use synapse_agentic::framework::{MemoryStore, MemoryFragment};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use std::sync::RwLock;
///
/// struct InMemoryStore {
///     data: RwLock<HashMap<String, MemoryFragment>>,
/// }
///
/// #[async_trait]
/// impl MemoryStore for InMemoryStore {
///     async fn store(&self, fragment: MemoryFragment) {
///         self.data.write().unwrap().insert(fragment.id.clone(), fragment);
///     }
///
///     async fn retrieve(&self, context: &str) -> Vec<MemoryFragment> {
///         self.data.read().unwrap()
///             .values()
///             .filter(|f| f.context == context)
///             .cloned()
///             .collect()
///     }
/// }
/// ```
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Stores a new memory fragment.
    async fn store(&self, fragment: MemoryFragment);

    /// Retrieves memory fragments by context.
    async fn retrieve(&self, context: &str) -> Vec<MemoryFragment>;

    /// Retrieves a specific fragment by ID.
    async fn get(&self, id: &str) -> Option<MemoryFragment> {
        // Default implementation - override for efficiency
        let _ = id;
        None
    }

    /// Deletes a fragment by ID.
    async fn delete(&self, id: &str) -> bool {
        // Default implementation - override for efficiency
        let _ = id;
        false
    }

    /// Searches memories by content (basic substring match).
    /// Override for semantic search capabilities.
    async fn search(&self, query: &str, limit: usize) -> Vec<MemoryFragment> {
        // Default: no search capability
        let _ = (query, limit);
        Vec::new()
    }
}
