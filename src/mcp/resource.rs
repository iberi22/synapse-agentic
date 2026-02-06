use async_trait::async_trait;

/// Represents a resource exposed by the MCP server.
/// Resources are file-like entities that can be read by the client.
#[async_trait]
pub trait Resource: Send + Sync {
    /// Returns the unique URI of the resource.
    fn uri(&self) -> &str;

    /// Returns the name of the resource.
    fn name(&self) -> &str;

    /// Returns the MIME type of the resource content.
    fn mime_type(&self) -> Option<&str>;

    /// Returns a description of the resource.
    fn description(&self) -> Option<&str>;

    /// Reads the content of the resource.
    async fn read(&self) -> anyhow::Result<String>;
}
