use chrono::NaiveDateTime;

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
	pub reading_time: u16,
	pub draft: bool
}

impl Post {
	#[must_use]
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
	pub tags: Vec<String>,
	pub draft: bool
}

#[must_use]
pub fn title_time_string(time: u64) -> String {
	time.try_into()
		.map_or_else(
			|_| "200 years in the future???".into(),
			|time| NaiveDateTime::from_timestamp_opt(time, 0)
				.map_or_else(
					|| "an unknown time".into(),
					|dt| dt.format("%H:%M on %b %-d, %Y").to_string()
				)
		)
}

pub static BASE_STYLE: &str = r"
* {
	--body-background: #3f3540;
	--main-text: #f1f6ff;
	--secondary-text: #f7ebec;
	--main-background: #1d1e2c;
	--secondary-background: #59656f;
	--border-color: #ac9fbb;
	--title-text: #d1bbe4;
	font-family: Arial;
	color: var(--main-text);
}
body {
	background-color: var(--body-background);
}
span.tag {
	margin-right: 8px;
	background-color: var(--secondary-background);
	padding: 4px 6px;
	border-radius: 4px;
}
input, textarea {
	background-color: var(--secondary-background);
	border: 1px solid var(--border-color);
	border-radius: 4px;
}
button {
	background-color: var(--main-background);
	border: 1px solid var(--main-background);
	border-radius: 4px;
	padding: 6px 8px;
}
pre {
	padding: 10px;
	border-radius: 8px;
	overflow: auto;
	-webkit-text-size-adjust: 140%;
}
pre > span, code {
	font-family: Courier;
}
";

pub static POST_LIST_STYLE: &str = r"
#posts {
	margin: 0px auto;
	max-width: max-content;
}
#title-text {
	max-width: max-content;
	display: inline-block;
	margin: 10px 0;
}
#home-title {
	display: flex;
	justify-content: space-between;
}
#home-title > a > svg {
	transform-origin: top center;
}
.post, #home-title, .page-selector {
	max-width: 900px;
}
#home-title {
	margin: 20px auto 10px auto;
}
.page-selector {
	margin: 10px auto;
}
#social-icons {
	align-self: center;
	display: inline-block;
}
.page-selector {
	margin: 12px auto;
	text-align: center;
}
a {
	text-decoration: none;
}
";
