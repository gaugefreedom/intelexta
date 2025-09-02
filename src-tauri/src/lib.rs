// Declare all the modules that make up our library and make them public
pub mod api;
pub mod store;
pub mod governance;
pub mod orchestrator;
pub mod provenance;
pub mod chunk;
pub mod ingest;

// Define the shared DbPool type and make it public so main.rs can see it
pub type DbPool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

