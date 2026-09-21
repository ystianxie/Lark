use crate::utils::dirs::app_data_dir;
use crate::utils::string_factory;
use anyhow::Result;
use pinyin::ToPinyin;
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use std::fmt::format;
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;

const RECORD_SQLITE_FILE: &str = "record_data_v1.sqlite";
const APP_FILE_INDEX_FILE: &str = "index_data_v1.sqlite";
static RECORD_SCHEMA_READY: OnceLock<()> = OnceLock::new();
static INDEX_SCHEMA_READY: OnceLock<()> = OnceLock::new();

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Record {
    pub id: u64,
    pub content: String,
    pub content_preview: Option<String>,
    pub data_type: String,
    pub md5: String,
    pub create_time: u64,
    pub app_icon: String,
    pub source: String,
    pub source_path: String,
}
impl Default for Record {
    fn default() -> Self {
        Self {
            id: 0,
            content: String::new(),
            content_preview: None,
            data_type: "text".to_string(),
            md5: String::new(),
            create_time: 0,
            app_icon: "".to_string(),
            source: "".to_string(),
            source_path: "".to_string(),
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Default, Clone)]
pub struct FileIndex {
    pub id: u64,
    pub title: String,
    pub path: String,
    pub desc: String,
    pub icon: String,
    pub pinyin: String,
    pub abb: String,
    pub file_type: String,
    pub md5: String,
    pub create_time: u64,
}

/// 文件系统增量变更，供 watcher 与索引数据库之间传递。
#[derive(Debug, Clone)]
pub enum FileIndexChange {
    Upsert(FileIndex),
    Remove {
        path: String,
        recursive: bool,
    },
    RemoveByType {
        file_type: String,
        roots: Vec<String>,
    },
    Rename {
        from: String,
        to: FileIndex,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct QueryReq {
    pub key: Option<String>,
    pub limit: Option<usize>,
}

pub struct RecordSQL {
    conn: Connection,
}

#[allow(unused)]
impl RecordSQL {
    pub fn new() -> Self {
        RECORD_SCHEMA_READY.get_or_init(Self::init);
        let data_dir = app_data_dir().unwrap().join(RECORD_SQLITE_FILE);
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        let _ = c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        RecordSQL { conn: c }
    }

    pub fn init() {
        // 创建数据库文件并连接及创建数据库
        let data_dir = app_data_dir().unwrap().join(RECORD_SQLITE_FILE);
        if !Path::new(&data_dir).exists() {
            File::create(&data_dir).unwrap();
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        let sql = r#"
        create table if not exists record
        (
            id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            content     TEXT,
            content_preview     TEXT,
            data_type   VARCHAR(20) DEFAULT '',
            md5         VARCHAR(200) DEFAULT '',
            source      VARCHAR(20) DEFAULT '',
            source_path TEXT NOT NULL DEFAULT '',
            create_time INTEGER
        );
        "#;
        c.execute(sql, ()).unwrap();
        let has_source_path = {
            let mut stmt = c.prepare("PRAGMA table_info(record)").unwrap();
            let columns = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
            let has_column = columns
                .filter_map(std::result::Result::ok)
                .any(|column| column == "source_path");
            has_column
        };
        if !has_source_path {
            // Add source identity to older databases without deleting or
            // rewriting their clipboard history.
            c.execute(
                "ALTER TABLE record ADD COLUMN source_path TEXT NOT NULL DEFAULT ''",
                (),
            )
            .unwrap();
        }
    }

    pub fn insert_record(&self, r: &Record) -> Result<i64> {
        let sql = "insert into record (content,md5,create_time,data_type,content_preview,source,source_path) values (?1,?2,?3,?4,?5,?6,?7)";
        let md5 = string_factory::md5(r.content.as_str());
        // SQLite INTEGER is a signed 64-bit value, which is the type rusqlite
        // accepts for integer parameters.
        let now = chrono::Local::now().timestamp_millis();
        let content_preview = r.content_preview.as_deref().unwrap_or("");
        let res = self.conn.execute(
            sql,
            (
                &r.content,
                md5,
                now,
                &r.data_type,
                content_preview,
                &r.source,
                &r.source_path,
            ),
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_record_by_md5(&self, md5: &str, data_type: &str) -> Result<Record> {
        let sql = "SELECT id FROM record WHERE md5 = ?1 and data_type = ?2";
        let r = self.conn.query_row(sql, [md5, data_type], |row| {
            Ok(Record {
                id: row.get::<_, i64>(0)? as u64,
                ..Default::default()
            })
        })?;
        Ok(r)
    }

    // 更新时间
    fn update_record_create_time(&self, r: &Record) -> Result<()> {
        let sql = "update record set create_time = ?2, source = ?3, source_path = ?4 where id = ?1";
        // 获取当前毫秒级时间戳
        let now = chrono::Local::now().timestamp_millis();
        self.conn.execute(
            sql,
            rusqlite::params![r.id as i64, now, &r.source, &r.source_path],
        )?;
        Ok(())
    }

    // 插入数据，如果存在则更新时间
    pub fn insert_if_not_exist(&self, r: &Record) -> Result<()> {
        let md5 = string_factory::md5(r.content.as_str());
        match self.find_record_by_md5(&md5, &r.data_type) {
            Ok(mut res) => {
                res.source = r.source.clone();
                res.source_path = r.source_path.clone();
                self.update_record_create_time(&res)?;
            }
            Err(_e) => {
                self.insert_record(r)?;
            }
        }
        Ok(())
    }

    pub fn md5_is_exist(&self, md5: &str) -> Result<bool> {
        let sql = "SELECT count(*) FROM record WHERE md5 = ?1";
        let count: u32 = self.conn.query_row(sql, [md5], |row| row.get(0))?;
        Ok(count > 0)
    }

    // 清除数据
    pub fn clear_data(&self) -> Result<()> {
        let sql = "delete from record";
        self.conn.execute(sql, ())?;
        Ok(())
    }

    pub fn find_all(&self) -> Result<Vec<Record>> {
        let sql = "SELECT id, content_preview, data_type, md5, create_time, source, source_path FROM record order by create_time desc";
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([])?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(2)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get::<_, i64>(0)? as u64,
                content,
                content_preview: None,
                data_type,
                md5: row.get(3)?,
                create_time: row.get::<_, i64>(4)? as u64,
                source: row.get(5)?,
                source_path: row.get(6)?,
                app_icon: "".to_string(),
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_part(&self, limit: i32, offset: i32) -> Result<Vec<Record>> {
        let sql = "SELECT id, content_preview, data_type, md5, create_time, source, source_path FROM record order by create_time desc limit ?1 offset ?2";
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([limit, offset])?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(2)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get::<_, i64>(0)? as u64,
                content,
                content_preview: None,
                data_type,
                md5: row.get(3)?,
                create_time: row.get::<_, i64>(4)? as u64,
                source: row.get(5)?,
                source_path: row.get(6)?,
                app_icon: "".to_string(),
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_by_key(&self, req: &QueryReq) -> Result<Vec<Record>> {
        let mut sql: String = String::new();
        sql.push_str(
            "SELECT id, content_preview, md5, create_time, data_type FROM record where 1=1",
        );
        let mut limit: usize = 300;
        let mut params: Vec<String> = vec![];
        if let Some(l) = req.limit {
            limit = l;
        }
        params.push(limit.to_string());
        if let Some(k) = &req.key {
            params.push(format!("%{}%", k));
            sql.push_str(
                format!(" and data_type='text' and content like ?{}", params.len()).as_str(),
            );
        }
        let sql = format!("{} order by create_time desc limit ?1", sql);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(4)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get::<_, i64>(0)? as u64,
                content,
                content_preview: None,
                data_type,
                md5: row.get(2)?,
                create_time: row.get::<_, i64>(3)? as u64,
                source: "".to_string(),
                source_path: "".to_string(),
                app_icon: "".to_string(),
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_by_keyword(&self, keyword: &str, offset: i32) -> Result<Vec<Record>> {
        let mut sql: String = String::new();
        sql.push_str(
            "SELECT id, content_preview, md5, create_time, data_type, source, source_path FROM record where and content like ?1",
        );
        let mut limit: usize = 30;
        let mut params: Vec<String> = vec![];
        params.push(format!("%{}%", keyword));
        params.push(limit.to_string());
        params.push(offset.to_string());
        let sql = format!("{} order by create_time desc limit ?2 offset ?3", sql);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(4)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get::<_, i64>(0)? as u64,
                content,
                content_preview: None,
                data_type,
                md5: row.get(2)?,
                create_time: row.get::<_, i64>(3)? as u64,
                source: row.get(5)?,
                source_path: row.get(6)?,
                app_icon: "".to_string(),
            };
            res.push(r);
        }
        Ok(res)
    }

    //删除超过limit的记录
    pub fn delete_over_limit(&self, limit: usize) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT count(id) FROM record")?;
        let mut rows = stmt.query([])?;
        // SQLite INTEGER maps to a signed 64-bit integer in rusqlite.
        let count: i64 = rows.next()?.unwrap().get(0)?;
        let limit = i64::try_from(limit)?;
        if count <= limit {
            return Ok(false);
        }
        let remove_num = count - limit;
        let sql = "DELETE FROM record WHERE id in (SELECT id FROM record order by create_time asc limit ?1)";
        self.conn.execute(sql, [remove_num])?;
        Ok(true)
    }

    pub fn delete_expired(&self, data_type: &str, days: i32) -> Result<bool> {
        let cutoff =
            chrono::Local::now().timestamp_millis() - i64::from(days) * 24 * 60 * 60 * 1000;
        let removed = self.conn.execute(
            "DELETE FROM record WHERE data_type = ?1 AND create_time < ?2",
            rusqlite::params![data_type, cutoff],
        )?;
        Ok(removed > 0)
    }

    pub fn find_by_id(&self, id: u64) -> Result<Record> {
        let sql =
            "SELECT id, content, data_type, md5, create_time, source, source_path FROM record where id = ?1";
        let r = self.conn.query_row(sql, [id as i64], |row| {
            Ok(Record {
                id: row.get::<_, i64>(0)? as u64,
                content: row.get(1)?,
                content_preview: None,
                data_type: row.get(2)?,
                md5: row.get(3)?,
                create_time: row.get::<_, i64>(4)? as u64,
                source: row.get(5)?,
                source_path: row.get(6)?,
                app_icon: "".to_string(),
            })
        })?;
        Ok(r)
    }
}

pub struct IndexSQL {
    conn: Connection,
}

fn ensure_app_index_custom_column(conn: &Connection) -> Result<()> {
    let has_is_custom = {
        let mut stmt = conn.prepare("PRAGMA table_info(app_index)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let x = columns
            .filter_map(std::result::Result::ok)
            .any(|column| column == "is_custom");
        x
    };
    if !has_is_custom {
        conn.execute(
            "ALTER TABLE app_index ADD COLUMN is_custom INTEGER NOT NULL DEFAULT 0",
            (),
        )?;
    }
    Ok(())
}

#[allow(unused)]
impl IndexSQL {
    pub fn new() -> Self {
        // 创建数据库链接
        let data_dir = app_data_dir().unwrap().join(APP_FILE_INDEX_FILE);
        Self::init();
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        let _ = c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        IndexSQL { conn: c }
    }

    pub fn has_app_indexes(&self) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM app_index", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn has_file_indexes(&self) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn init() {
        INDEX_SCHEMA_READY.get_or_init(|| {
        // 创建数据库文件并连接及创建数据库
        let data_dir = app_data_dir().unwrap().join(APP_FILE_INDEX_FILE);
        if !Path::new(&data_dir).exists() {
            println!("创建数据库文件:{:?}", &data_dir);
            File::create(&data_dir).unwrap();
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
        let _ = c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        let sql = r#"
        CREATE TABLE IF NOT EXISTS app_index
        (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title        TEXT DEFAULT '',
            path        TEXT NOT NULL UNIQUE,
            desc        TEXT DEFAULT '',
            icon        TEXT DEFAULT '',
            pinyin      TEXT DEFAULT '',
            abb         TEXT DEFAULT '',
            type        TEXT DEFAULT 'app',
            md5         TEXT NOT NULL,
            is_custom   INTEGER NOT NULL DEFAULT 0,
            create_time INTEGER DEFAULT (strftime('%s', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_md5 ON app_index (md5);
        "#;
        c.execute_batch(sql).unwrap();
        // Migrate databases created before manual application entries were introduced.
        ensure_app_index_custom_column(&c).unwrap();
        let sql = r#"
        CREATE TABLE IF NOT EXISTS file_index
        (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title        TEXT DEFAULT '',
            path        TEXT NOT NULL UNIQUE,
            desc        TEXT DEFAULT '',
            icon        TEXT DEFAULT '',
            pinyin      TEXT DEFAULT '',
            abb         TEXT DEFAULT '',
            type        TEXT DEFAULT 'app',
            md5         TEXT NOT NULL,
            generation  INTEGER NOT NULL DEFAULT 0,
            create_time INTEGER DEFAULT (strftime('%s', 'now'))
        );
        "#;
        c.execute_batch(sql).unwrap();
        // Migrate databases created before snapshot generations were introduced.
        let _ = c.execute(
            "ALTER TABLE file_index ADD COLUMN generation INTEGER NOT NULL DEFAULT 0",
            (),
        );
        c.execute_batch(r#"
            CREATE INDEX IF NOT EXISTS idx_md5 ON file_index (md5);
            CREATE INDEX IF NOT EXISTS idx_file_generation_title ON file_index (generation, title);
            CREATE INDEX IF NOT EXISTS idx_file_generation_path ON file_index (generation, path);
            CREATE INDEX IF NOT EXISTS idx_file_title_nocase ON file_index (title COLLATE NOCASE);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_file_index_path_unique ON file_index (path);
            CREATE TABLE IF NOT EXISTS file_index_staging
            (
                title TEXT DEFAULT '', path TEXT NOT NULL UNIQUE, desc TEXT DEFAULT '',
                pinyin TEXT DEFAULT '', abb TEXT DEFAULT '', type TEXT DEFAULT 'file', md5 TEXT NOT NULL
            );
        "#).unwrap();
        });
    }

    pub fn insert_file_index(&self, r: &FileIndex) -> Result<i64> {
        let sql = "insert into file_index (title,path,desc,type,md5) values (?1,?2,?3,?4,?5)";
        let md5 = string_factory::md5(r.path.as_str());
        let res = self
            .conn
            .execute(sql, [&r.title, &r.path, &r.desc, &r.file_type, &md5]);
        match res {
            Ok(r) => {}
            Err(e) => {
                println!("插入索引失败:{:?}", e);
            }
        }
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_file_indexes(&mut self, paths: Vec<FileIndex>) -> Result<()> {
        println!("开始提交索引:{:?}", &paths.len());
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT OR IGNORE INTO file_index (title,path,desc,pinyin,abb,type,md5,generation) VALUES (?1,?2,?3,?4,?5,?6,?7,0)")?;
            for path in paths {
                let md5 = string_factory::md5(path.path.as_str());
                let res = stmt.execute(rusqlite::params![
                    path.title,
                    path.path,
                    path.desc,
                    path.pinyin,
                    path.abb,
                    path.file_type,
                    md5
                ]);
                match res {
                    Ok(r) => {}
                    Err(e) => {
                        println!("插入索引失败:{:?}", e);
                    }
                }
            }
        }
        tx.commit()?; // 提交事务
        Ok(())
    }

    /// 在一个事务内应用一批增量文件索引变更。
    /// 全量 generation 重建仍使用 staging 接口；两者不应并发调用。
    pub fn apply_file_changes(&mut self, changes: &[FileIndexChange]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for change in changes {
            match change {
                FileIndexChange::Upsert(file) => {
                    let md5 = string_factory::md5(&file.path);
                    tx.execute(
                        "INSERT INTO file_index (title,path,desc,icon,pinyin,abb,type,md5,generation) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0) ON CONFLICT(path) DO UPDATE SET title=excluded.title, desc=excluded.desc, icon=excluded.icon, pinyin=excluded.pinyin, abb=excluded.abb, type=excluded.type, md5=excluded.md5",
                        rusqlite::params![file.title, file.path, file.desc, file.icon, file.pinyin, file.abb, file.file_type, md5],
                    )?;
                }
                FileIndexChange::Remove { path, recursive } => {
                    if *recursive {
                        let base = path.trim_end_matches(['\\', '/']);
                        let win_prefix = format!(r#"{}\%"#, base);
                        let unix_prefix = format!("{}/%", base);
                        tx.execute(
                            "DELETE FROM file_index WHERE path = ?1 OR path LIKE ?2 OR path LIKE ?3",
                            rusqlite::params![path, win_prefix, unix_prefix],
                        )?;
                    } else {
                        tx.execute("DELETE FROM file_index WHERE path = ?1", [path])?;
                    }
                }
                FileIndexChange::RemoveByType { file_type, roots } => {
                    for root in roots {
                        let base = root.trim_end_matches(['\\', '/']);
                        let win_prefix = format!(r#"{}\%"#, base);
                        let unix_prefix = format!("{}/%", base);
                        tx.execute("DELETE FROM file_index WHERE type = ?1 AND (path = ?2 OR path = ?3 OR path LIKE ?4 OR path LIKE ?5)", rusqlite::params![file_type, base, base, win_prefix, unix_prefix])?;
                    }
                }
                FileIndexChange::Rename { from, to } => {
                    tx.execute("DELETE FROM file_index WHERE path = ?1", [from])?;
                    let md5 = string_factory::md5(&to.path);
                    tx.execute(
                        "INSERT INTO file_index (title,path,desc,icon,pinyin,abb,type,md5,generation) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0) ON CONFLICT(path) DO UPDATE SET title=excluded.title, desc=excluded.desc, icon=excluded.icon, pinyin=excluded.pinyin, abb=excluded.abb, type=excluded.type, md5=excluded.md5",
                        rusqlite::params![to.title, to.path, to.desc, to.icon, to.pinyin, to.abb, to.file_type, md5],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn begin_file_generation(&self) -> Result<i64> {
        let generation: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(generation), 0) + 1 FROM file_index",
            [],
            |row| row.get(0),
        )?;
        self.conn.execute("DELETE FROM file_index_staging", [])?;
        Ok(generation)
    }

    pub fn insert_file_generation(&mut self, _generation: i64, paths: &[FileIndex]) -> Result<()> {
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare("INSERT OR REPLACE INTO file_index_staging (title,path,desc,pinyin,abb,type,md5) VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
        for path in paths {
            let md5 = string_factory::md5(&path.path);
            stmt.execute(rusqlite::params![
                path.title,
                path.path,
                path.desc,
                path.pinyin,
                path.abb,
                path.file_type,
                md5
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn commit_file_generation(&mut self, generation: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM file_index", [])?;
        tx.execute("INSERT INTO file_index (title,path,desc,pinyin,abb,type,md5,generation) SELECT title,path,desc,pinyin,abb,type,md5,?1 FROM file_index_staging", [generation])?;
        tx.execute("DELETE FROM file_index_staging", [])?;
        tx.commit()?;
        Ok(())
    }

    pub fn insert_app_index(&self, r: &FileIndex) -> Result<i64> {
        let sql = "insert into app_index (title,path,desc,icon,pinyin,abb,md5) values (?1,?2,?3,?4,?5,?6,?7)";
        let md5 = string_factory::md5(r.path.as_str());
        let res = self.conn.execute(
            sql,
            [
                &r.title,
                &r.path,
                &r.desc,
                &r.icon,
                &r.pinyin,
                &r.abb,
                &r.file_type,
                &md5,
            ],
        );
        match res {
            Ok(r) => {}
            Err(e) => {
                println!("插入索引失败:{:?}", e);
            }
        }
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_app_indexes(&mut self, paths: Vec<FileIndex>) -> Result<()> {
        println!("开始提交索引:{:?}", &paths.len());
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO app_index (title, path, desc, icon, pinyin, abb, md5)
                SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7
                WHERE NOT EXISTS (
                    SELECT 1 FROM app_index
                    WHERE replace(lower(path), '/', '\') = replace(lower(?2), '/', '\')
                )
                "#,
            )?;
            for r in paths {
                let md5 = string_factory::md5(r.path.as_str());
                let params = &[&r.title, &r.path, &r.desc, &r.icon, &r.pinyin, &r.abb, &md5];
                let res = stmt.execute(params);
                match res {
                    Ok(_) => {
                        // println!("插入索引成功");
                    }
                    Err(e) => {
                        println!("插入索引失败:{:?}", e);
                    }
                }
            }
        }
        tx.commit()?; // 提交事务
        Ok(())
    }

    pub fn replace_discovered_app_indexes(&mut self, paths: Vec<FileIndex>) -> Result<usize> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM app_index WHERE is_custom = 0", [])?;
        let inserted = {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO app_index (title, path, desc, icon, pinyin, abb, type, md5, is_custom)
                SELECT ?1, ?2, ?3, ?4, ?5, ?6, 'app', ?7, 0
                WHERE NOT EXISTS (
                    SELECT 1 FROM app_index
                    WHERE replace(lower(path), '/', '\') = replace(lower(?2), '/', '\')
                )
                "#,
            )?;
            let mut inserted = 0;
            for app in paths {
                let md5 = string_factory::md5(&app.path);
                inserted += stmt.execute(rusqlite::params![
                    app.title, app.path, app.desc, app.icon, app.pinyin, app.abb, md5,
                ])?;
            }
            inserted
        };
        tx.commit()?;
        Ok(inserted)
    }

    pub fn upsert_custom_app_index(&mut self, app: &FileIndex) -> Result<()> {
        let md5 = string_factory::md5(&app.path);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing_id = tx
            .query_row(
                r#"
                SELECT id FROM app_index
                WHERE replace(lower(path), '/', '\') = replace(lower(?1), '/', '\')
                ORDER BY is_custom DESC, id
                LIMIT 1
                "#,
                [&app.path],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(id) = existing_id {
            tx.execute(
                r#"
                DELETE FROM app_index
                WHERE id <> ?1
                  AND replace(lower(path), '/', '\') = replace(lower(?2), '/', '\')
                "#,
                rusqlite::params![id, &app.path],
            )?;
            let updated = tx.execute(
                r#"
                UPDATE app_index SET
                    title = ?1, path = ?2, desc = ?3, icon = ?4, pinyin = ?5,
                    abb = ?6, type = 'app', md5 = ?7, is_custom = 1
                WHERE id = ?8
                "#,
                rusqlite::params![
                    &app.title,
                    &app.path,
                    &app.desc,
                    &app.icon,
                    &app.pinyin,
                    &app.abb,
                    &md5,
                    id
                ],
            )?;
            if updated != 1 {
                return Err(anyhow::anyhow!("自定义应用索引更新失败"));
            }
        } else {
            tx.execute(
                r#"
                INSERT INTO app_index (title, path, desc, icon, pinyin, abb, type, md5, is_custom)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'app', ?7, 1)
                "#,
                rusqlite::params![
                    &app.title,
                    &app.path,
                    &app.desc,
                    &app.icon,
                    &app.pinyin,
                    &app.abb,
                    md5
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_custom_app_indexes(&self) -> Result<Vec<FileIndex>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, path, desc, icon FROM app_index WHERE is_custom = 1 ORDER BY title COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FileIndex {
                id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                path: row.get(2)?,
                desc: row.get(3)?,
                icon: row.get(4)?,
                file_type: "app".to_string(),
                ..Default::default()
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_custom_app_index(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM app_index WHERE id = ?1 AND is_custom = 1",
            [id],
        )?;
        Ok(())
    }

    pub fn clear_discovered_app_indexes(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM app_index WHERE is_custom = 0", [])?;
        Ok(())
    }

    pub fn find_app(&self, keyword: &str, offset: i32) -> Result<Vec<FileIndex>> {
        let mut sql: String = String::new();
        sql.push_str(
            "SELECT id, title, path, desc, icon FROM app_index
             WHERE (lower(title) LIKE lower(?1) OR lower(pinyin) LIKE lower(?2)
             OR lower(abb) LIKE lower(?2) OR lower(path) LIKE lower(?3))",
        );
        let mut limit: usize = 30;
        let mut params: Vec<String> = vec![];
        params.push(format!("{}%", keyword));
        params.push(format!("{}%", keyword));
        params.push(format!("%{}%", keyword));
        params.push(limit.to_string());
        params.push(offset.to_string());
        let sql = format!(
            "{} ORDER BY
             CASE
               WHEN lower(title) = lower(?1) THEN 0
               WHEN lower(title) LIKE lower(?1) THEN 1
               WHEN instr(replace(lower(path), '\\', '/'), '/' || lower(title) || '/') > 0 THEN 2
               WHEN lower(path) LIKE lower(?3) THEN 3
               ELSE 4
             END,
             length(title), title COLLATE NOCASE
             LIMIT ?4 OFFSET ?5",
            sql
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let r = FileIndex {
                id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                path: row.get(2)?,
                desc: row.get(3)?,
                icon: row.get(4)?,
                file_type: "app".to_string(),
                ..Default::default()
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_app_icon(&self, app_name: &str, app_path: &str) -> Result<FileIndex> {
        let sql = r#"
            SELECT id, title, path, icon
            FROM app_index
            WHERE (
                ?2 <> ''
                AND replace(lower(path), '/', '\') = replace(lower(?2), '/', '\')
            ) OR lower(title) = lower(?1)
            ORDER BY CASE
                WHEN ?2 <> ''
                    AND replace(lower(path), '/', '\') = replace(lower(?2), '/', '\')
                THEN 0
                ELSE 1
            END
            LIMIT 1
        "#;
        let r = self
            .conn
            .query_row(sql, rusqlite::params![app_name, app_path], |row| {
                Ok(FileIndex {
                    id: row.get::<_, i64>(0)? as u64,
                    title: row.get(1)?,
                    path: row.get(2)?,
                    icon: row.get(3)?,
                    ..Default::default()
                })
            })
            .unwrap_or(FileIndex::default());
        Ok(r)
    }

    pub fn find_by_id(&self, table: &str, id: i64) -> Result<FileIndex> {
        let sql = &format!(
            "SELECT id, title, path, type FROM {}_index where id = ?1",
            table
        );
        let r = self.conn.query_row(sql, [&id], |row| {
            Ok(FileIndex {
                id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                path: row.get(2)?,
                file_type: row.get(3)?,
                ..Default::default()
            })
        })?;
        Ok(r)
    }

    fn query_file_search_layer(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<FileIndex>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            Ok(FileIndex {
                id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                path: row.get(2)?,
                desc: row.get(3)?,
                icon: row.get(4)?,
                file_type: row.get(5)?,
                ..Default::default()
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn find_by_keyword(
        &self,
        table: &str,
        keyword: &str,
        offset: i32,
    ) -> Result<Vec<FileIndex>> {
        const LIMIT: usize = 30;
        if table != "file" {
            anyhow::bail!("layered keyword search only supports file_index");
        }

        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Ok(Vec::new());
        }

        // Escape LIKE metacharacters so user input is treated as literal text.
        let escaped = keyword
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let prefix = format!("{}%", escaped);
        let contains = format!("%{}%", escaped);
        let offset = offset.max(0) as usize;
        let target_count = offset.saturating_add(LIMIT);
        let mut results = Vec::with_capacity(target_count);
        let mut remaining = target_count as i64;

        // Layer 1: exact title. This is index-friendly and preserves the strongest match.
        let exact = self.query_file_search_layer(
            "SELECT id, title, path, desc, icon, type FROM file_index \
             WHERE title = ?1 COLLATE NOCASE \
             ORDER BY length(title), title COLLATE NOCASE LIMIT ?2",
            &[&keyword, &remaining],
        )?;
        remaining -= exact.len() as i64;
        results.extend(exact);

        // Layer 2: title prefix, excluding the exact-title layer.
        if remaining > 0 {
            let rows = self.query_file_search_layer(
                "SELECT id, title, path, desc, icon, type FROM file_index \
                 WHERE title COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
                   AND title <> ?2 COLLATE NOCASE \
                 ORDER BY length(title), title COLLATE NOCASE LIMIT ?3",
                &[&prefix, &keyword, &remaining],
            )?;
            remaining -= rows.len() as i64;
            results.extend(rows);
        }

        // A one-character substring/path search is extremely broad. Keep it responsive by
        // returning only exact and prefix title matches until the user types another character.
        if keyword.chars().count() < 2 {
            return Ok(results.into_iter().skip(offset).take(LIMIT).collect());
        }

        // Layer 3: title substring, excluding all prefix matches already considered above.
        if remaining > 0 {
            let rows = self.query_file_search_layer(
                "SELECT id, title, path, desc, icon, type FROM file_index \
                 WHERE title COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
                   AND title COLLATE NOCASE NOT LIKE ?2 ESCAPE '\\' \
                 ORDER BY length(title), title COLLATE NOCASE LIMIT ?3",
                &[&contains, &prefix, &remaining],
            )?;
            remaining -= rows.len() as i64;
            results.extend(rows);
        }

        // Layer 4: path substring only when title matching still did not fill the page.
        if remaining > 0 {
            let rows = self.query_file_search_layer(
                "SELECT id, title, path, desc, icon, type FROM file_index \
                 WHERE path COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
                   AND title COLLATE NOCASE NOT LIKE ?1 ESCAPE '\\' \
                 ORDER BY length(title), title COLLATE NOCASE LIMIT ?2",
                &[&contains, &remaining],
            )?;
            results.extend(rows);
        }

        Ok(results.into_iter().skip(offset).take(LIMIT).collect())
    }

    pub fn delete_by_id(&self, table: &str, id: i64) -> Result<()> {
        let sql = &format!("delete from {}_index where id = ?1", table);
        self.conn.execute(sql, [id.to_string()])?;
        Ok(())
    }

    pub fn insert_if_not_exist(&self, table: &str, r: &FileIndex) -> Result<()> {
        let md5 = string_factory::md5(r.path.as_str());
        match self.find_by_md5(table, &md5) {
            Ok(res) => {
                self.update_create_time(table, &res)?;
            }
            Err(_e) => {
                self.insert_file_index(r)?;
            }
        }
        Ok(())
    }

    pub fn md5_is_exist(&self, table: &str, md5: &str) -> Result<bool> {
        let sql = &format!("SELECT count(*) FROM {}_index WHERE md5 = ?1", table);
        let count: u32 = self
            .conn
            .query_row(sql, [md5.to_string()], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn clear_data(&self, table: &str) -> Result<()> {
        let sql = &format!("delete from {}_index", table);
        self.conn.execute(sql, ())?;
        Ok(())
    }

    fn find_by_md5(&self, table: &str, md5: &str) -> Result<FileIndex> {
        let sql = &format!("SELECT id FROM {}_index WHERE md5 = ?1", table);
        let r = self.conn.query_row(sql, [md5.to_string()], |row| {
            Ok(FileIndex {
                id: row.get::<_, i64>(0)? as u64,
                ..Default::default()
            })
        })?;
        Ok(r)
    }

    fn update_create_time(&self, table: &str, r: &FileIndex) -> Result<()> {
        let sql = &format!("update {}_index set create_time = ?1 where id = ?2", table);
        // 获取当前毫秒级时间戳
        let now = chrono::Local::now().timestamp_millis() as u64;
        self.conn
            .execute(sql, [now.to_string(), r.id.to_string()])?;
        Ok(())
    }
}

#[cfg(test)]
mod file_search_tests {
    use super::*;

    fn test_index(rows: &[(&str, &str)]) -> IndexSQL {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE file_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT DEFAULT '', path TEXT NOT NULL UNIQUE, desc TEXT DEFAULT '',
                icon TEXT DEFAULT '', type TEXT DEFAULT 'file'
            );
            CREATE INDEX idx_file_title_nocase ON file_index (title COLLATE NOCASE);
            "#,
        )
        .unwrap();
        for (title, path) in rows {
            conn.execute(
                "INSERT INTO file_index (title, path, desc, type) VALUES (?1, ?2, ?2, 'file')",
                [title, path],
            )
            .unwrap();
        }
        IndexSQL { conn }
    }

    #[test]
    fn incremental_changes_upsert_remove_and_rename() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE file_index (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT, path TEXT NOT NULL UNIQUE, desc TEXT, icon TEXT, pinyin TEXT, abb TEXT, type TEXT, md5 TEXT NOT NULL, generation INTEGER NOT NULL DEFAULT 0, create_time INTEGER);").unwrap();
        let mut db = IndexSQL { conn };
        let file = FileIndex {
            title: "a.txt".into(),
            path: "a.txt".into(),
            file_type: "txt".into(),
            ..Default::default()
        };
        db.apply_file_changes(&[FileIndexChange::Upsert(file.clone())])
            .unwrap();
        db.apply_file_changes(&[FileIndexChange::Rename {
            from: "a.txt".into(),
            to: FileIndex {
                path: "b.txt".into(),
                title: "b.txt".into(),
                ..file.clone()
            },
        }])
        .unwrap();
        db.apply_file_changes(&[FileIndexChange::Remove {
            path: "b.txt".into(),
            recursive: false,
        }])
        .unwrap();
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn single_character_search_only_returns_exact_and_title_prefix_matches() {
        let db = test_index(&[
            ("a", r"C:\root\a"),
            ("Alpha.txt", r"C:\root\Alpha.txt"),
            ("Beta.txt", r"C:\root\Beta.txt"),
            ("notes.txt", r"C:\archive\notes.txt"),
        ]);

        let results = db.find_by_keyword("file", "a", 0).unwrap();
        let titles: Vec<_> = results.iter().map(|item| item.title.as_str()).collect();

        assert_eq!(titles, vec!["a", "Alpha.txt"]);
    }

    #[test]
    fn multi_character_search_falls_back_from_title_to_path_contains() {
        let db = test_index(&[
            ("Alpha.txt", r"C:\root\Alpha.txt"),
            ("Graph.txt", r"C:\root\Graph.txt"),
            ("notes.txt", r"C:\alpha-folder\notes.txt"),
        ]);

        let results = db.find_by_keyword("file", "ph", 0).unwrap();
        let titles: Vec<_> = results.iter().map(|item| item.title.as_str()).collect();

        assert_eq!(titles, vec!["Alpha.txt", "Graph.txt", "notes.txt"]);
    }

    #[test]
    fn like_wildcards_in_user_input_are_matched_literally() {
        let db = test_index(&[
            ("100% real.txt", r"C:\root\100% real.txt"),
            ("1000 real.txt", r"C:\root\1000 real.txt"),
        ]);

        let results = db.find_by_keyword("file", "100%", 0).unwrap();
        let titles: Vec<_> = results.iter().map(|item| item.title.as_str()).collect();

        assert_eq!(titles, vec!["100% real.txt"]);
    }
}

#[cfg(test)]
mod app_index_tests {
    use super::*;

    fn test_index() -> IndexSQL {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE app_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT DEFAULT '', path TEXT NOT NULL UNIQUE, desc TEXT DEFAULT '',
                icon TEXT DEFAULT '', pinyin TEXT DEFAULT '', abb TEXT DEFAULT '',
                type TEXT DEFAULT 'app', md5 TEXT NOT NULL,
                is_custom INTEGER NOT NULL DEFAULT 0,
                create_time INTEGER DEFAULT (strftime('%s', 'now'))
            );
            "#,
        )
        .unwrap();
        IndexSQL { conn }
    }

    #[test]
    fn old_app_index_schema_migrates_without_losing_rows() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE app_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT DEFAULT '', path TEXT NOT NULL UNIQUE, md5 TEXT NOT NULL
            );
            INSERT INTO app_index (title, path, md5) VALUES ('Existing', 'C:\existing.exe', 'a');
            "#,
        )
        .unwrap();

        ensure_app_index_custom_column(&conn).unwrap();

        let row: (String, i64) = conn
            .query_row("SELECT title, is_custom FROM app_index", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(row, ("Existing".to_string(), 0));
    }

    #[test]
    fn rebuild_cleanup_preserves_only_custom_apps() {
        let db = test_index();
        db.conn
            .execute(
                "INSERT INTO app_index (title, path, md5, is_custom) VALUES ('Auto', 'C:\\auto.exe', 'a', 0), ('Custom', 'C:\\custom.exe', 'b', 1)",
                [],
            )
            .unwrap();

        db.clear_discovered_app_indexes().unwrap();

        let rows = db.list_custom_app_indexes().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Custom");
    }

    #[test]
    fn replacing_discovered_apps_is_atomic_and_preserves_custom_priority() {
        let mut db = test_index();
        db.conn
            .execute(
                "INSERT INTO app_index (title, path, md5, is_custom) VALUES ('Old auto', 'C:\\old.exe', 'a', 0), ('Custom', 'C:\\custom.exe', 'b', 1)",
                [],
            )
            .unwrap();

        db.replace_discovered_app_indexes(vec![
            FileIndex {
                title: "New auto".to_string(),
                path: r"C:\new.exe".to_string(),
                ..Default::default()
            },
            FileIndex {
                title: "Scanner duplicate".to_string(),
                path: r"c:/CUSTOM.exe".to_string(),
                ..Default::default()
            },
        ])
        .unwrap();

        let rows: Vec<(String, String, i64)> = db
            .conn
            .prepare("SELECT title, path, is_custom FROM app_index ORDER BY is_custom DESC, title")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "Custom");
        assert_eq!(rows[0].2, 1);
        assert_eq!(rows[1].0, "New auto");
        assert_eq!(rows[1].2, 0);
    }

    #[test]
    fn custom_app_is_updated_and_only_explicitly_deleted() {
        let mut db = test_index();
        let mut app = FileIndex {
            title: "First name".to_string(),
            path: r"C:\custom.exe".to_string(),
            desc: r"C:\custom.exe".to_string(),
            ..Default::default()
        };
        db.upsert_custom_app_index(&app).unwrap();
        app.title = "Updated name".to_string();
        app.path = r"c:/CUSTOM.exe".to_string();
        db.upsert_custom_app_index(&app).unwrap();

        let mut scanner = test_index();
        scanner.upsert_custom_app_index(&app).unwrap();
        scanner
            .insert_app_indexes(vec![FileIndex {
                title: "Scanner duplicate".to_string(),
                path: r"C:\custom.exe".to_string(),
                ..Default::default()
            }])
            .unwrap();
        let scanner_count: i64 = scanner
            .conn
            .query_row("SELECT COUNT(*) FROM app_index", [], |row| row.get(0))
            .unwrap();
        assert_eq!(scanner_count, 1);

        let rows = db.list_custom_app_indexes().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Updated name");

        db.delete_custom_app_index(rows[0].id as i64).unwrap();
        assert!(db.list_custom_app_indexes().unwrap().is_empty());
    }
}

#[cfg(test)]
mod clipboard_retention_tests {
    use super::*;

    fn test_records() -> RecordSQL {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE record (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT, content_preview TEXT, data_type TEXT,
                md5 TEXT, source TEXT, source_path TEXT,
                create_time INTEGER
            );
            "#,
        )
        .unwrap();
        RecordSQL { conn }
    }

    #[test]
    fn count_limit_is_exact_and_type_expiry_is_independent() {
        let db = test_records();
        let now = chrono::Local::now().timestamp_millis();
        let old = now - 3 * 24 * 60 * 60 * 1000;
        db.conn
            .execute_batch(&format!(
                r#"
                INSERT INTO record (content, data_type, md5, create_time) VALUES
                    ('old text', 'text', '1', {old}),
                    ('old image', 'image', '2', {old}),
                    ('new text', 'text', '3', {now}),
                    ('new file', 'file', '4', {now});
                "#
            ))
            .unwrap();

        assert!(db.delete_expired("text", 1).unwrap());
        let old_image_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM record WHERE content = 'old image'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_image_count, 1);

        assert!(db.delete_over_limit(2).unwrap());
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM record", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}

#[test]
#[allow(unused)]
fn test_sqlite_insert() {
    RecordSQL::init();
    println!("{:?}", IndexSQL::new().find_app("wec", 0));
    let r = Record {
        content: "1234567".to_string(),
        md5: "e10adc3949ba59abbe56e057f20f8823e".to_string(),
        create_time: 12345689,
        ..Default::default()
    };
    let q = QueryReq {
        key: Option::from("123456".to_string()),
        ..Default::default()
    };
    // println!("{:?}",SqliteDB::new().md5_is_exist("e10adc3949ba59abbe56e057f20f883e").unwrap());
    // println!("{:?}",SqliteDB::new().find_all());
    // println!("{:?}",SqliteDB::new().clear_data());
    // println!("{:?}",SqliteDB::new().find_by_key(&q).unwrap());
    // println!("{:?}", RecordSQL::new().find_by_id(3).unwrap());
    // assert_eq!(SqliteDB::new().insert_record(&r).unwrap(), 1_i64)
}
