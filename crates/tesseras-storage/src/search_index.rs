use std::sync::{Arc, Mutex};

use tesseras_core::enums::Visibility;
use tesseras_core::ports::SearchIndex;
use tesseras_core::search::{MetadataExcerpt, SearchFilters, SearchHit};
use tesseras_core::types::ContentHash;
use tesseras_core::CoreError;

/// SQLite-backed search index using FTS5 and R-tree.
pub struct SqliteSearchIndex {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteSearchIndex {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }
}

impl SearchIndex for SqliteSearchIndex {
    fn index_tessera(
        &self,
        hash: &ContentHash,
        title: Option<&str>,
        description: Option<&str>,
        memory_type: Option<&str>,
        language: Option<&str>,
        tags: &[String],
        visibility: &str,
        created_at: &chrono::DateTime<chrono::Utc>,
        lat: Option<f64>,
        lon: Option<f64>,
    ) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        let hash_str = hash.to_string();
        let tags_str = tags.join(",");

        conn.execute(
            "INSERT OR REPLACE INTO search_index (tessera_hash, title, description, memory_type, language, tags, created_at, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                hash_str,
                title.unwrap_or(""),
                description.unwrap_or(""),
                memory_type.unwrap_or(""),
                language.unwrap_or(""),
                tags_str,
                created_at.to_rfc3339(),
                visibility,
            ],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;

        // Index geo if coordinates provided
        if let (Some(lat), Some(lon)) = (lat, lon) {
            // Get or create geo map entry
            conn.execute(
                "INSERT OR REPLACE INTO search_index_geo_map (tessera_hash)
                 VALUES (?1)",
                rusqlite::params![hash_str],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

            let geo_id: i64 = conn
                .query_row(
                    "SELECT id FROM search_index_geo_map WHERE tessera_hash = ?1",
                    rusqlite::params![hash_str],
                    |row| row.get(0),
                )
                .map_err(|e| CoreError::Database(e.to_string()))?;

            conn.execute(
                "INSERT OR REPLACE INTO search_index_geo (id, min_lat, max_lat, min_lon, max_lon)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![geo_id, lat, lat, lon, lon],
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;
        }

        Ok(())
    }

    fn remove_tessera(&self, hash: &ContentHash) -> Result<(), CoreError> {
        let conn = self.conn.lock().unwrap();
        let hash_str = hash.to_string();

        conn.execute(
            "DELETE FROM search_index WHERE tessera_hash = ?1",
            rusqlite::params![hash_str],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;

        // Clean up geo data
        conn.execute(
            "DELETE FROM search_index_geo WHERE id IN (
                SELECT id FROM search_index_geo_map WHERE tessera_hash = ?1
            )",
            rusqlite::params![hash_str],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;
        conn.execute(
            "DELETE FROM search_index_geo_map WHERE tessera_hash = ?1",
            rusqlite::params![hash_str],
        )
        .map_err(|e| CoreError::Database(e.to_string()))?;

        Ok(())
    }

    fn search(
        &self,
        query: &str,
        filters: &SearchFilters,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<SearchHit>, u64), CoreError> {
        let conn = self.conn.lock().unwrap();

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // FTS5 text match
        if !query.is_empty() {
            conditions.push("search_index MATCH ?".to_string());
            params.push(Box::new(query.to_string()));
        }

        // Memory type filter
        if let Some(ref mt) = filters.memory_type {
            conditions.push("memory_type = ?".to_string());
            params.push(Box::new(format!("{mt:?}").to_lowercase()));
        }

        // Language filter
        if let Some(ref lang) = filters.language {
            conditions.push("language = ?".to_string());
            params.push(Box::new(lang.clone()));
        }

        // Tags filter (comma-separated LIKE match)
        if let Some(ref tags) = filters.tags {
            for tag in tags {
                conditions.push("tags LIKE ?".to_string());
                params.push(Box::new(format!("%{tag}%")));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM search_index {where_clause}");
        let total: u64 = conn
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |row| row.get(0),
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        // Fetch page
        let offset = page * page_size;
        let select_sql = format!(
            "SELECT tessera_hash, title, description, memory_type, language, tags, created_at, visibility
             FROM search_index {where_clause}
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        );

        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if !query.is_empty() {
            all_params.push(Box::new(query.to_string()));
        }
        if let Some(ref mt) = filters.memory_type {
            all_params.push(Box::new(format!("{mt:?}").to_lowercase()));
        }
        if let Some(ref lang) = filters.language {
            all_params.push(Box::new(lang.clone()));
        }
        if let Some(ref tags) = filters.tags {
            for tag in tags {
                all_params.push(Box::new(format!("%{tag}%")));
            }
        }
        all_params.push(Box::new(page_size));
        all_params.push(Box::new(offset));

        let mut stmt = conn
            .prepare(&select_sql)
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())),
                |row| {
                    let hash_str: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let description: String = row.get(2)?;
                    let _memory_type_str: String = row.get(3)?;
                    let language: String = row.get(4)?;
                    let tags_str: String = row.get(5)?;
                    let _created_at: String = row.get(6)?;
                    let _visibility_str: String = row.get(7)?;
                    Ok((hash_str, title, description, language, tags_str))
                },
            )
            .map_err(|e| CoreError::Database(e.to_string()))?;

        let mut hits = Vec::new();
        for row in rows {
            let (hash_str, title, description, language, tags_str) =
                row.map_err(|e| CoreError::Database(e.to_string()))?;

            let hash: ContentHash = hash_str
                .parse()
                .map_err(|_| CoreError::Database(format!("invalid hash: {hash_str}")))?;

            let tags: Vec<String> = if tags_str.is_empty() {
                vec![]
            } else {
                tags_str.split(',').map(|s| s.to_string()).collect()
            };

            hits.push(SearchHit {
                hash,
                metadata: MetadataExcerpt {
                    title: if title.is_empty() { None } else { Some(title) },
                    description: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    memory_type: None,
                    created_at: None,
                    visibility: Visibility::Public,
                    language: if language.is_empty() {
                        None
                    } else {
                        Some(language)
                    },
                    tags,
                },
            });
        }

        Ok((hits, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_migrations;

    fn setup() -> SqliteSearchIndex {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        SqliteSearchIndex::new(Arc::new(Mutex::new(conn)))
    }

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }

    #[test]
    fn index_and_search_by_text() {
        let idx = setup();
        let now = chrono::Utc::now();
        idx.index_tessera(
            &hash(0x01),
            Some("São Paulo memories"),
            Some("Daily life in the city"),
            Some("daily"),
            Some("pt-BR"),
            &["cotidiano".into(), "cidade".into()],
            "public",
            &now,
            Some(-23.5505),
            Some(-46.6333),
        )
        .unwrap();

        let (hits, total) = idx
            .search("São Paulo", &SearchFilters::default(), 0, 10)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hash, hash(0x01));
        assert_eq!(
            hits[0].metadata.title,
            Some("São Paulo memories".into())
        );
    }

    #[test]
    fn search_empty_query_returns_all() {
        let idx = setup();
        let now = chrono::Utc::now();
        for i in 0..3u8 {
            idx.index_tessera(
                &hash(i),
                Some(&format!("Memory {i}")),
                None,
                None,
                None,
                &[],
                "public",
                &now,
                None,
                None,
            )
            .unwrap();
        }

        let (hits, total) = idx
            .search("", &SearchFilters::default(), 0, 10)
            .unwrap();
        assert_eq!(total, 3);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_pagination() {
        let idx = setup();
        let now = chrono::Utc::now();
        for i in 0..5u8 {
            idx.index_tessera(
                &hash(i),
                Some(&format!("Memory {i}")),
                None,
                None,
                None,
                &[],
                "public",
                &now,
                None,
                None,
            )
            .unwrap();
        }

        let (hits, total) = idx
            .search("", &SearchFilters::default(), 0, 2)
            .unwrap();
        assert_eq!(total, 5);
        assert_eq!(hits.len(), 2);

        let (hits2, _) = idx
            .search("", &SearchFilters::default(), 1, 2)
            .unwrap();
        assert_eq!(hits2.len(), 2);
    }

    #[test]
    fn remove_tessera_from_index() {
        let idx = setup();
        let now = chrono::Utc::now();
        idx.index_tessera(
            &hash(0x01),
            Some("Test"),
            None,
            None,
            None,
            &[],
            "public",
            &now,
            Some(1.0),
            Some(2.0),
        )
        .unwrap();

        idx.remove_tessera(&hash(0x01)).unwrap();

        let (hits, total) = idx
            .search("", &SearchFilters::default(), 0, 10)
            .unwrap();
        assert_eq!(total, 0);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_by_language_filter() {
        let idx = setup();
        let now = chrono::Utc::now();
        idx.index_tessera(
            &hash(0x01),
            Some("English"),
            None,
            None,
            Some("en"),
            &[],
            "public",
            &now,
            None,
            None,
        )
        .unwrap();
        idx.index_tessera(
            &hash(0x02),
            Some("Portuguese"),
            None,
            None,
            Some("pt-BR"),
            &[],
            "public",
            &now,
            None,
            None,
        )
        .unwrap();

        let filters = SearchFilters {
            language: Some("pt-BR".into()),
            ..Default::default()
        };
        let (hits, total) = idx.search("", &filters, 0, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(hits[0].hash, hash(0x02));
    }
}
