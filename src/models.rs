use super::schema::{files, sites};
use chrono::NaiveDateTime;
use diesel::{sqlite::Sqlite, Identifiable, Insertable, Queryable, Selectable};
use serde::Deserialize;

#[derive(Queryable, Insertable, Identifiable, Deserialize, Selectable)]
#[diesel(check_for_backend(Sqlite))]
#[diesel(table_name = sites)]
pub struct Site {
    pub id: String,
    pub host: String,
    pub index_file: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Insertable, Identifiable, Deserialize, Selectable)]
#[diesel(belongs_to(Site, foreign_key= site_id))]
#[diesel(check_for_backend(Sqlite))]
#[diesel(table_name = files)]
pub struct File {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub size: i64,
    pub is_index: bool,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
