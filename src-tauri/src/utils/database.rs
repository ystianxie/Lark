use std::fmt::format;
use crate::utils::dirs::app_data_dir;
use crate::utils::string_factory;
use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::fs::File;
use std::path::Path;
use pinyin::ToPinyin;

const RECORD_SQLITE_FILE: &str = "record_data_v1.sqlite";
const APP_FILE_INDEX_FILE: &str = "index_data_v1.sqlite";

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Record {
    pub id: u64,
    pub content: String,
    pub content_preview: Option<String>,
    pub data_type: String,
    pub md5: String,
    pub create_time: u64,
    pub app_icon:String,
    pub source: String,
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
        // 创建数据库链接
        let data_dir = app_data_dir().unwrap().join(RECORD_SQLITE_FILE);
        if !Path::new(&data_dir).exists() {
            Self::init()
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        RecordSQL { conn: c }
    }

    pub fn init() {
        // 创建数据库文件并连接及创建数据库
        let data_dir = app_data_dir().unwrap().join(RECORD_SQLITE_FILE);
        if !Path::new(&data_dir).exists() {
            File::create(&data_dir).unwrap();
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        let sql = r#"
        create table if not exists record
        (
            id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            content     TEXT,
            content_preview     TEXT,
            data_type   VARCHAR(20) DEFAULT '',
            md5         VARCHAR(200) DEFAULT '',
            source      VARCHAR(20) DEFAULT '',
            create_time INTEGER
        );
        "#;
        c.execute(sql, ()).unwrap();
    }

    pub fn insert_record(&self, r: &Record) -> Result<i64> {
        let sql = "insert into record (content,md5,create_time,data_type,content_preview,source) values (?1,?2,?3,?4,?5,?6)";
        let md5 = string_factory::md5(r.content.as_str());
        let now = chrono::Local::now().timestamp_millis() as u64;
        let content_preview = r.content_preview.as_deref().unwrap_or("");
        let res = self.conn.execute(sql, (&r.content, md5, now, &r.data_type, content_preview, &r.source))?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_record_by_md5(&self, md5: &str, data_type: &str) -> Result<Record> {
        let sql = "SELECT id FROM record WHERE md5 = ?1 and data_type = ?2";
        let r = self.conn.query_row(sql, [md5, data_type], |row| {
            Ok(Record { id: row.get(0)?, ..Default::default() })
        })?;
        Ok(r)
    }

    // 更新时间
    fn update_record_create_time(&self, r: &Record) -> Result<()> {
        let sql = "update record set create_time = ?2 where id = ?1";
        // 获取当前毫秒级时间戳
        let now = chrono::Local::now().timestamp_millis() as u64;
        self.conn.execute(sql, [&r.id, &now])?;
        Ok(())
    }

    // 插入数据，如果存在则更新时间
    pub fn insert_if_not_exist(&self, r: &Record) -> Result<()> {
        let md5 = string_factory::md5(r.content.as_str());
        match self.find_record_by_md5(&md5, &r.data_type) {
            Ok(res) => {
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
        let sql = "SELECT id, content_preview, data_type, md5, create_time, source FROM record order by create_time desc";
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([])?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(2)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get(0)?,
                content,
                content_preview: None,
                data_type,
                md5: row.get(3)?,
                create_time: row.get(4)?,
                source: row.get(5)?,
                app_icon:"".to_string()
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_part(&self, limit: i32, offset: i32) -> Result<Vec<Record>> {
        let sql = "SELECT id, content_preview, data_type, md5, create_time, source FROM record order by create_time desc limit ?1 offset ?2";
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([limit, offset])?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let data_type: String = row.get(2)?;
            let content: String = row.get(1)?;
            let r = Record {
                id: row.get(0)?,
                content,
                content_preview: None,
                data_type,
                md5: row.get(3)?,
                create_time: row.get(4)?,
                source: row.get(5)?,
                app_icon:"".to_string()
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
                id: row.get(0)?,
                content,
                content_preview: None,
                data_type,
                md5: row.get(2)?,
                create_time: row.get(3)?,
                source: "".to_string(),
                app_icon:"".to_string()
            };
            res.push(r);
        }
        Ok(res)
    }

    pub fn find_by_keyword(&self, keyword: &str, offset: i32) -> Result<Vec<Record>> {
        let mut sql: String = String::new();
        sql.push_str(
            "SELECT id, content_preview, md5, create_time, data_type, source FROM record where and content like ?1",
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
                id: row.get(0)?,
                content,
                content_preview: None,
                data_type,
                md5: row.get(2)?,
                create_time: row.get(3)?,
                source: row.get(5)?,
                app_icon:"".to_string()
            };
            res.push(r);
        }
        Ok(res)
    }

    //删除超过limit的记录
    pub fn delete_over_limit(&self, limit: usize) -> Result<bool> {
        // 先查询count，如果数量超过limit 10个以上了就删除多余的部分 主要是防止频繁重建数据库
        let mut stmt = self.conn.prepare("SELECT count(id) FROM record")?;
        let mut rows = stmt.query([])?;
        let count: usize = rows.next()?.unwrap().get(0).unwrap();
        if count < 10 + limit {
            return Ok(false);
        }
        let remove_num = count - limit;
        let sql = "DELETE FROM record WHERE id in (SELECT id FROM record order by create_time asc limit ?1)";
        self.conn.execute(sql, [remove_num])?;
        Ok(true)
    }

    pub fn find_by_id(&self, id: u64) -> Result<Record> {
        let sql = "SELECT id, content, data_type, md5, create_time, source FROM record where id = ?1";
        let r = self.conn.query_row(sql, [&id], |row| {
            Ok(Record {
                id: row.get(0)?,
                content: row.get(1)?,
                content_preview: None,
                data_type: row.get(2)?,
                md5: row.get(3)?,
                create_time: row.get(4)?,
                source: row.get(5)?,
                app_icon:"".to_string()
            })
        })?;
        Ok(r)
    }
}

pub struct IndexSQL {
    conn: Connection,
}

#[allow(unused)]
impl IndexSQL {
    pub fn new() -> Self {
        // 创建数据库链接
        let data_dir = app_data_dir().unwrap().join(APP_FILE_INDEX_FILE);
        // let data_dir = "/Users/starsxu/.config/lark/data/index_data_v1.sqlite";
        if !Path::new(&data_dir).exists() {
            Self::init()
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        IndexSQL { conn: c }
    }

    pub fn init() {
        // 创建数据库文件并连接及创建数据库
        let data_dir = app_data_dir().unwrap().join(APP_FILE_INDEX_FILE);
        if !Path::new(&data_dir).exists() {
            println!("创建数据库文件:{:?}", &data_dir);
            File::create(&data_dir).unwrap();
        }
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
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
            create_time INTEGER DEFAULT (strftime('%s', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_md5 ON app_index (md5);
        "#;
        c.execute(sql, ()).unwrap();
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
            create_time INTEGER DEFAULT (strftime('%s', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_md5 ON file_index (md5);
        "#;
        c.execute(sql, ()).unwrap();
    }

    pub fn insert_file_index(&self, r: &FileIndex) -> Result<i64> {
        let sql = "insert into file_index (title,path,desc,type,md5) values (?1,?2,?3,?4,?5)";
        let md5 = string_factory::md5(r.path.as_str());
        let res = self.conn.execute(
            sql, [&r.title, &r.path, &r.desc, &r.file_type, &md5],
        );
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
            let mut stmt = tx.prepare("INSERT OR IGNORE INTO file_index (title,path,desc,type,md5) VALUES (?1,?2,?3,?4,?5)")?;
            for path in paths {
                let md5 = string_factory::md5(path.path.as_str());
                let res = stmt.execute(&[&path.title, &path.path, &path.desc, &path.file_type, &md5]);
                match res {
                    Ok(r) => {}
                    Err(e) => { println!("插入索引失败:{:?}", e); }
                }
            }
        }
        tx.commit()?; // 提交事务
        Ok(())
    }

    pub fn insert_app_index(&self, r: &FileIndex) -> Result<i64> {
        let sql = "insert into app_index (title,path,desc,icon,pinyin,abb,md5) values (?1,?2,?3,?4,?5,?6,?7)";
        let md5 = string_factory::md5(r.path.as_str());
        let res = self.conn.execute(
            sql, [&r.title, &r.path, &r.desc, &r.icon, &r.pinyin, &r.abb, &r.file_type, &md5],
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
            let mut stmt = tx.prepare("INSERT OR IGNORE INTO app_index (title,path,desc,icon,pinyin,abb,md5) VALUES (?1,?2,?3,?4,?5,?6,?7)")?;
            for r in paths {
                let md5 = string_factory::md5(r.path.as_str());
                let params = &[&r.title, &r.path, &r.desc, &r.icon, &r.pinyin, &r.abb, &md5];
                let res = stmt.execute(params);
                match res {
                    Ok(_) => {
                        // println!("插入索引成功");
                    }
                    Err(e) => { println!("插入索引失败:{:?}", e); }
                }
            }
        }
        tx.commit()?; // 提交事务
        Ok(())
    }

    pub fn find_app(&self, keyword: &str, offset: i32) -> Result<Vec<FileIndex>> {
        let mut sql: String = String::new();
        sql.push_str(
            "SELECT id, title, path, desc, icon FROM app_index where (title like ?1 or pinyin like ?2 or abb like ?2 or path like ?3)"
        );
        let mut limit: usize = 30;
        let mut params: Vec<String> = vec![];
        params.push(format!("{}%", keyword));
        params.push(format!("{}%", keyword));
        params.push(format!("%/Applications/{}%.app", keyword));
        params.push(limit.to_string());
        params.push(offset.to_string());
        let sql = format!("{} order by create_time desc limit ?4 offset ?5", sql);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
        let mut res = vec![];
        while let Some(row) = rows.next()? {
            let r = FileIndex {
                id: row.get(0)?,
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

    pub fn find_app_icon(&self, app_name: &str) -> Result<FileIndex> {
        let mut sql = "SELECT id, title, icon FROM app_index where title = ?1";
        let r = self.conn.query_row(sql, [app_name], |row| {
            Ok(FileIndex {
                id: row.get(0)?,
                title: row.get(1)?,
                icon: row.get(2)?,
                ..Default::default()
            })
        }).unwrap_or(FileIndex::default());
        Ok(r)
    }

    pub fn find_by_id(&self, table: &str, id: i64) -> Result<FileIndex> {
        let sql = &format!("SELECT id, title, path, type FROM {}_index where id = ?1", table);
        let r = self.conn.query_row(sql, [&id], |row| {
            Ok(FileIndex {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                file_type: row.get(3)?,
                ..Default::default()
            })
        })?;
        Ok(r)
    }

    pub fn find_by_keyword(&self, table: &str, keyword: &str, offset: i32) -> Result<Vec<FileIndex>> {
        let mut sql: String = String::new();
        sql.push_str(
            &format!("SELECT id, title, path, desc, icon, type FROM {}_index where title like ?1", table)
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
            let r = FileIndex {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                desc: row.get(3)?,
                icon: row.get(4)?,
                file_type: row.get(5)?,
                ..Default::default()
            };
            res.push(r);
        }
        Ok(res)
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
        let count: u32 = self.conn.query_row(sql, [md5.to_string()], |row| row.get(0))?;
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
            Ok(FileIndex { id: row.get(0)?, ..Default::default() })
        })?;
        Ok(r)
    }

    fn update_create_time(&self, table: &str, r: &FileIndex) -> Result<()> {
        let sql = &format!("update {}_index set create_time = ?1 where id = ?2", table);
        // 获取当前毫秒级时间戳
        let now = chrono::Local::now().timestamp_millis() as u64;
        self.conn.execute(sql, [now.to_string(), r.id.to_string()])?;
        Ok(())
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
