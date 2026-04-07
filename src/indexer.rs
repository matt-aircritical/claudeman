use crate::session::Session;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, QueryParser, TermQuery};
use tantivy::schema::{
    IndexRecordOption, Schema, SchemaBuilder, TextFieldIndexing, TextOptions, Value, FAST, INDEXED,
    STORED, STRING,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, Order, ReloadPolicy, Term};

#[derive(Debug, Clone)]
pub struct IndexedSession {
    pub session_id: String,
    pub project_dir: String,
    pub cwd: String,
    pub started_at: u64,
    pub last_activity: u64,
    pub name: String,
    pub model: String,
    pub version: String,
    pub message_count: u32,
    pub first_user_message: String,
    pub first_assistant_message: String,
    pub jsonl_path: String,
}

pub struct SessionIndex {
    index: Index,
    reader: IndexReader,
    schema: Schema,
    writer: Mutex<IndexWriter>,
}

impl SessionIndex {
    pub fn create(path: &Path) -> Result<Self> {
        let schema = build_schema();
        let index = match Index::create_in_dir(path, schema.clone()) {
            Ok(idx) => idx,
            Err(_) => Index::open_in_dir(path)?,
        };
        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        Ok(SessionIndex {
            index,
            reader,
            schema,
            writer: Mutex::new(writer),
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(path)?;
        let schema = index.schema();
        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        Ok(SessionIndex {
            index,
            reader,
            schema,
            writer: Mutex::new(writer),
        })
    }

    pub fn add_session(&self, session: &Session) -> Result<()> {
        let writer = self.writer.lock().map_err(|e| anyhow!("lock error: {e}"))?;

        // Delete existing document with same session_id
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let delete_term = Term::from_field_text(session_id_field, &session.session_id);
        writer.delete_term(delete_term);

        let project_dir_field = self.schema.get_field("project_dir").unwrap();
        let cwd_field = self.schema.get_field("cwd").unwrap();
        let started_at_field = self.schema.get_field("started_at").unwrap();
        let last_activity_field = self.schema.get_field("last_activity").unwrap();
        let name_field = self.schema.get_field("name").unwrap();
        let model_field = self.schema.get_field("model").unwrap();
        let version_field = self.schema.get_field("version").unwrap();
        let message_count_field = self.schema.get_field("message_count").unwrap();
        let user_messages_field = self.schema.get_field("user_messages").unwrap();
        let assistant_messages_field = self.schema.get_field("assistant_messages").unwrap();
        let first_user_message_field = self.schema.get_field("first_user_message").unwrap();
        let first_assistant_message_field =
            self.schema.get_field("first_assistant_message").unwrap();
        let file_mtime_field = self.schema.get_field("file_mtime").unwrap();
        let jsonl_path_field = self.schema.get_field("jsonl_path").unwrap();

        let started_at_ts = session.started_at.timestamp() as u64;
        let last_activity_ts = session.last_activity.timestamp() as u64;

        writer.add_document(doc!(
            session_id_field => session.session_id.clone(),
            project_dir_field => session.project_dir.clone(),
            cwd_field => session.cwd.clone(),
            started_at_field => started_at_ts,
            last_activity_field => last_activity_ts,
            name_field => session.name.clone(),
            model_field => session.model.clone(),
            version_field => session.version.clone(),
            message_count_field => session.message_count as u64,
            user_messages_field => session.user_messages.clone(),
            assistant_messages_field => session.assistant_messages.clone(),
            first_user_message_field => session.first_user_message.clone(),
            first_assistant_message_field => session.first_assistant_message.clone(),
            file_mtime_field => session.file_mtime,
            jsonl_path_field => session.jsonl_path.clone()
        ))?;

        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|e| anyhow!("lock error: {e}"))?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn all_sessions(&self) -> Result<Vec<IndexedSession>> {
        let searcher = self.reader.searcher();

        let collector = TopDocs::with_limit(10_000)
            .order_by_u64_field("last_activity", Order::Desc);
        let results: Vec<(Option<u64>, tantivy::DocAddress)> =
            searcher.search(&AllQuery, &collector)?;

        let mut sessions = Vec::new();
        for (_score, doc_address) in results {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            sessions.push(doc_to_indexed_session(&doc, &self.schema));
        }
        Ok(sessions)
    }

    pub fn search(&self, query_str: &str) -> Result<Vec<IndexedSession>> {
        let searcher = self.reader.searcher();

        let name_field = self.schema.get_field("name").unwrap();
        let user_messages_field = self.schema.get_field("user_messages").unwrap();
        let assistant_messages_field = self.schema.get_field("assistant_messages").unwrap();
        let project_dir_field = self.schema.get_field("project_dir").unwrap();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                name_field,
                user_messages_field,
                assistant_messages_field,
                project_dir_field,
            ],
        );

        let query = query_parser.parse_query(query_str)?;
        let results: Vec<(tantivy::Score, tantivy::DocAddress)> =
            searcher.search(&query, &TopDocs::with_limit(100).order_by_score())?;

        let mut sessions = Vec::new();
        for (_score, doc_address) in results {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            sessions.push(doc_to_indexed_session(&doc, &self.schema));
        }
        Ok(sessions)
    }

    pub fn needs_reindex(&self, session_id: &str, current_mtime: u64) -> Result<bool> {
        let searcher = self.reader.searcher();
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let file_mtime_field = self.schema.get_field("file_mtime").unwrap();

        let term = Term::from_field_text(session_id_field, session_id);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let results: Vec<(tantivy::Score, tantivy::DocAddress)> =
            searcher.search(&query, &TopDocs::with_limit(1).order_by_score())?;

        if results.is_empty() {
            return Ok(true);
        }

        let (_score, doc_address) = results[0];
        let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

        let stored_mtime = doc
            .get_first(file_mtime_field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(stored_mtime != current_mtime)
    }

    pub fn remove_session(&self, session_id: &str) -> Result<()> {
        let writer = self.writer.lock().map_err(|e| anyhow!("lock error: {e}"))?;
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let term = Term::from_field_text(session_id_field, session_id);
        writer.delete_term(term);
        Ok(())
    }
}

fn build_schema() -> Schema {
    let mut builder: SchemaBuilder = Schema::builder();

    // session_id: exact match (STRING), stored
    builder.add_text_field("session_id", STRING | STORED);

    // project_dir: full-text searchable + stored
    builder.add_text_field("project_dir", tantivy::schema::TEXT | STORED);

    // cwd: stored only
    builder.add_text_field("cwd", STORED);

    // timestamps: u64 indexed + stored + fast (fast field needed for order_by_u64_field)
    builder.add_u64_field("started_at", INDEXED | STORED);
    builder.add_u64_field("last_activity", INDEXED | STORED | FAST);

    // name: full-text + stored
    builder.add_text_field("name", tantivy::schema::TEXT | STORED);

    // model, version: stored only
    builder.add_text_field("model", STORED);
    builder.add_text_field("version", STORED);

    // message_count: stored only (u64)
    builder.add_u64_field("message_count", STORED);

    // user_messages, assistant_messages: full-text indexed, NOT stored
    let text_indexed_only = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    builder.add_text_field("user_messages", text_indexed_only.clone());
    builder.add_text_field("assistant_messages", text_indexed_only);

    // first messages: stored only
    builder.add_text_field("first_user_message", STORED);
    builder.add_text_field("first_assistant_message", STORED);

    // file_mtime: u64 stored
    builder.add_u64_field("file_mtime", STORED);

    // jsonl_path: stored only
    builder.add_text_field("jsonl_path", STORED);

    builder.build()
}

fn doc_to_indexed_session(doc: &tantivy::TantivyDocument, schema: &Schema) -> IndexedSession {
    let get_str = |name: &str| -> String {
        let field = schema.get_field(name).unwrap();
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let get_u64 = |name: &str| -> u64 {
        let field = schema.get_field(name).unwrap();
        doc.get_first(field).and_then(|v| v.as_u64()).unwrap_or(0)
    };

    IndexedSession {
        session_id: get_str("session_id"),
        project_dir: get_str("project_dir"),
        cwd: get_str("cwd"),
        started_at: get_u64("started_at"),
        last_activity: get_u64("last_activity"),
        name: get_str("name"),
        model: get_str("model"),
        version: get_str("version"),
        message_count: get_u64("message_count") as u32,
        first_user_message: get_str("first_user_message"),
        first_assistant_message: get_str("first_assistant_message"),
        jsonl_path: get_str("jsonl_path"),
    }
}
