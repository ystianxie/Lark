use crate::utils::dirs::app_data_dir;
use crate::utils::string_factory;
use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::fs::File;
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Record {
    pub id: u64,
    pub content: String,
    pub content_preview: Option<String>,
    pub data_type: String,
    pub md5: String,
    pub create_time: u64,
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
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct QueryReq {
    pub key: Option<String>,
    pub limit: Option<usize>,
}

pub struct SqliteDB {
    conn: Connection,
}

const SQLITE_FILE: &str = "clipboard_data_v1.sqlite";

#[allow(unused)]
impl SqliteDB {
    pub fn new() -> Self {
        // 创建数据库链接
        let data_dir = app_data_dir().unwrap().join(SQLITE_FILE);
        let c = Connection::open_with_flags(data_dir, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
        SqliteDB { conn: c }
    }

    pub fn init() {
        // 创建数据库文件并连接及创建数据库
        let data_dir = app_data_dir().unwrap().join(SQLITE_FILE);
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
            create_time INTEGER
        );
        "#;
        c.execute(sql, ()).unwrap();
    }

    pub fn insert_record(&self, r: &Record) -> Result<i64> {
        let sql = "insert into record (content,md5,create_time,data_type,content_preview) values (?1,?2,?3,?4,?5)";
        let md5 = string_factory::md5(r.content.as_str());
        let now = chrono::Local::now().timestamp_millis() as u64;
        let content_preview = r.content_preview.as_deref().unwrap_or("");
        let res = self.conn.execute(
            sql,
            (
                &r.content,
                md5,
                now,
                &r.data_type,
                content_preview,
            ),
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn find_record_by_md5(&self, md5: &str, data_type: &str) -> Result<Record> {
        let sql = "SELECT id FROM record WHERE md5 = ?1 and data_type = ?2";
        let r = self.conn.query_row(sql, [md5, data_type], |row| {
            Ok(Record {
                id: row.get(0)?,
                ..Default::default()
            })
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
        let sql = "SELECT id, content_preview, data_type, md5, create_time FROM record order by create_time desc";
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
        let sql = "SELECT id, content, data_type, md5, create_time FROM record where id = ?1";
        let r = self.conn.query_row(sql, [&id], |row| {
            Ok(Record {
                id: row.get(0)?,
                content: row.get(1)?,
                content_preview: None,
                data_type: row.get(2)?,
                md5: row.get(3)?,
                create_time: row.get(4)?,
            })
        })?;
        Ok(r)
    }
}

#[test]
#[allow(unused)]
fn test_sqlite_insert() {
    SqliteDB::init();
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
    println!("{:?}", SqliteDB::new().find_by_id(3).unwrap());
    // assert_eq!(SqliteDB::new().insert_record(&r).unwrap(), 1_i64)
}
