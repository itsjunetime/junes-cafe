use std::collections::HashSet;

use chrono::DateTime;

mod md_to_html;
pub use md_to_html::md_to_html;

pub const GITHUB_ICON: &str = include_str!("../../assets/github-mark.svg");
pub const BLUESKY_ICON: &str = include_str!("../../assets/bluesky.svg");
pub const MATRIX_ICON: &str = include_str!("../../assets/matrix.svg");
pub const RSS_ICON: &str = include_str!("../../assets/rss-icon.svg");

#[cfg(feature = "sqlx")]
use sqlx::{Row, FromRow, postgres::PgRow};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
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
	pub fn display_user(username: &str) -> &str {
		if username.is_empty() {
			"Unknown"
		} else {
			username
		}
	}
}

// We can thankfully just derive Deserialize for this because when it's returned
// through JSON, it'll be given to us with the tags in an array, not a string,
// but when it's given from a row, it'll be in text.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Tags(pub Vec<String>);

#[cfg(feature = "sqlx")]
impl FromRow<'_, PgRow> for Tags {
	fn from_row(row: &PgRow) -> sqlx::Result<Self> {
		row.try_get::<&str, _>("tags")
			.map(|t| Self(
				t.split(',')
					.filter(|t| !t.is_empty())
					.map(str::to_string)
					.collect()
			))
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PostReq {
	pub id: Option<u32>,
	pub title: String,
	pub content: String,
	pub tags: HashSet<String>,
	pub draft: bool
}

#[must_use]
pub fn title_time_string(time: u64) -> String {
	time.try_into()
		.map_or_else(
			|_| "200 years in the future???".into(),
			|time| DateTime::from_timestamp(time, 0)
				.map_or_else(
					|| "an unknown time".into(),
					|dt| dt.naive_utc().format("%H:%M on %b %-d, %Y").to_string()
				)
		)
}

pub static BASE_STYLE: &str = r##"
@font-face {
	font-family: "Maple Mono";
	src: local("Maple Mono"), url("/font/maple-mono");
	font-display: swap;
}
* {
	--body-background: #31242b;
	--main-text: #fbebe2;
	--secondary-text: #ffd8f0;
	--main-background: #3c2c35;
	--secondary-background: #59656f;
	--border-color: #a16d8f;
	--title-text: #d1bbe4;
	--code-background: #2a1e24;
	color: var(--main-text);
	font-family: "Maple Mono", monospace,monospace;
	font-weight: 300;
}
body {
	background-color: var(--body-background);
}
span.tag {
	background-color: var(--secondary-background);
	padding: 6px 6px 2px 6px;
	border-radius: 8px 0;
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
pre span, code {
	font-family: "Maple Mono", monospace,monospace;
	font-weight: lighter;
}
pre, code {
	background-color: var(--code-background);
}
code {
	padding: 1px 6px 2px 6px;
	border-radius: 4px;
	line-break: loose;
}
pre > code {
	padding: 0px;
}
p > code {
	color: var(--title-text);
}
blockquote {
	opacity: 0.8;
	border-left: 3px solid var(--border-color);
	padding-left: 20px;
	margin-left: 0px;
}
.tag-group {
	display: flex;
	column-gap: 8px;
	overflow: auto;
}
#post-text h2::before {
	content: "#";
	position: relative;
	left: -30px;
	max-width: 0px;
	display: inline-block;
	color: coral;
}
"##;

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
#credits {
	max-width: 900px;
	margin: auto;
	text-align: center;
	color: var(--title-text);
}
#credits a {
	color: var(--title-text);
	text-decoration: underline;
}
.tag-group {
	justify-content: space-between;
}
";
