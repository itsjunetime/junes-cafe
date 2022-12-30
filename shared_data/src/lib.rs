// re-export so others can use as well
#[cfg(feature = "sqlx")]
pub use sqlx;

#[cfg(feature = "sqlx")]
use sqlx::{Row, FromRow, postgres::PgRow};

#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Post {
	#[cfg_attr(feature = "sqlx", sqlx(try_from = "i32"))]
	pub id: u32,
	#[cfg_attr(feature = "sqlx", sqlx(default))]
	pub username: String,
	#[cfg_attr(feature = "sqlx", sqlx(try_from = "i64"))]
	pub created_at: u64,
	pub title: String,
	pub html: String,
	pub orig_markdown: String,
	#[cfg_attr(feature = "sqlx", sqlx(flatten))]
	pub tags: Tags,
	#[cfg_attr(feature = "sqlx", sqlx(try_from = "i32"))]
	pub reading_time: u16
}

impl Post {
	pub fn display_user(&self) -> &str {
		if self.username.is_empty() {
			"Unknown"
		} else {
			self.username.as_str()
		}
	}
}

// We can thankfully just derive Deserialize for this because when it's returned
// through JSON, it'll be given to us with the tags in an array, not a string,
// but when it's given from a row, it'll be in text.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Tags(pub Vec<String>);

#[cfg(feature = "sqlx")]
impl FromRow<'_, PgRow> for Tags {
	fn from_row(row: &PgRow) -> sqlx::Result<Self> {
		row.try_get::<&str, _>("tags")
			.map(|t| Self(if t.is_empty() {
				Vec::new()
			} else {
				t.split(',').map(str::to_string).collect()
			}))
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PostReq {
	pub title: String,
	pub content: String,
	pub tags: Vec<String>
}
