use super::schema::todos;
use chrono::NaiveDateTime;
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Insertable, Identifiable, Serialize, Deserialize, Selectable)]
#[diesel(table_name = todos)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub completed_at: Option<NaiveDateTime>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
