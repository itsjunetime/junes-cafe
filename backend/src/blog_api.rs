use tower_cache::invalidator::Invalidator;
use tower_sessions::Session;
use axum_sqlx_tx::Tx;
use sqlx::{Postgres, query_as, query, Row};
use axum::{http::StatusCode, extract::Path, response::Json};
use shared_data::{Post, PostReq};
use backend::{auth::get_username, check_auth};

use crate::print_and_ret;

use std::time::{SystemTime, UNIX_EPOCH};

pub async fn get_post_list(
	session: Option<Session>,
	tx: &mut Tx<Postgres>,
	count: u32,
	offset: u32,
) -> Result<Vec<Post>, sqlx::Error> {
	let username = match session {
		Some(s) => get_username(&s).await,
		None => None
	};
	get_post_list_with_user(tx, count, offset, username).await
}

pub async fn get_post_list_with_user(
	tx: &mut Tx<Postgres>,
	count: u32,
	offset: u32,
	username: Option<String>,
) -> Result<Vec<Post>, sqlx::Error> {
	let username_clause = username.as_ref().map_or("", |_| "u.username = $1 OR");

	let q_string = format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, p.draft, u.username \
		FROM posts p \
		LEFT JOIN users u ON u.id = p.created_by_user \
		WHERE {username_clause} p.draft IS NOT TRUE \
		ORDER BY id DESC \
		LIMIT {count} \
		OFFSET {offset} \
	;");
	let mut q = query_as::<_, Post>(&q_string);

	if let Some(name) = username {
		q = q.bind(name);
	}

	q.fetch_all(tx).await
}

pub async fn get_post(
	session: Session,
	mut tx: Tx<Postgres>,
	Path(id): Path<i32>
) -> Result<Post, sqlx::Error> {
	// If they're logged in, they should be able to view drafts
	let username = get_username(&session).await;
	let where_clause = if username.is_some() {
		// If they're signed in with a specific username, they should be able to see all the draft
		// posts for that specific username; not others.
		"WHERE (p.id = $1 AND (p.draft IS NOT TRUE OR u.username = $2))"
	} else {
		"WHERE (p.id = $1 AND p.draft IS NOT TRUE)"
	};

	let query_str = format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, p.draft, u.username \
		FROM \
		posts p LEFT JOIN users u ON u.id = p.created_by_user \
		{where_clause}\
	;");

	let mut q = query_as::<_, Post>(&query_str)
		.bind(id);

	if let Some(name) = username {
		q = q.bind(name);
	}

	q.fetch_one(&mut tx).await
}

pub async fn get_post_json(
	session: Session,
	tx: Tx<Postgres>,
	path: Path<i32>
) -> Result<Json<Post>, (StatusCode, String)> {
	get_post(session, tx, path)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't get post: {e:?}");
			match e {
				sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Post not found".into()),
				_ => (StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't retrieve post: {e:?}"))
			}
		})
}

pub async fn submit_post(
	session: Session,
	mut tx: Tx<Postgres>,
	// inval: Invalidator,
	Json(payload): Json<PostReq>
) -> (StatusCode, String) {
	let username = check_auth!(session);

	println!("New post being submitted by user {username}");

	let details = post_details(payload);

	if details.content.is_empty() || details.title.is_empty() {
		return (StatusCode::BAD_REQUEST, "The title or content of the post are empty".into());
	}

	// Because the UNIX_EPOCH is inherently UTC, the timestamp is for UTC
	// I know this conversion (`as i64`) is not safe, technically, but I doubt this site
	// will survive long enough that we'll get a timestamp higher than i64::MAX
	let Ok(created_at) = SystemTime::now().duration_since(UNIX_EPOCH).map(|c| c.as_secs() as i64) else {
		print_and_ret!("Time has gone backwards?? what the fuck")
	};

	// we're just assuming the average wpm for these articles is 220
	let minutes = details.reading_time();

	query("INSERT INTO posts
		(created_by_user, created_at, title, html, orig_markdown, tags, reading_time, draft)
		SELECT id, $1, $2, $3, $4, $5, $6, $7 FROM users WHERE username = $8
		RETURNING id
	;").bind(created_at)
		.bind(details.title)
		.bind(details.html)
		.bind(details.content)
		.bind(details.tags)
		.bind(minutes as i32)
		.bind(details.draft)
		.bind(username)
		.fetch_one(&mut tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Failed to create new post: {e}"),
			|r| r.try_get::<i32, _>("id")
				.map_or_else(
					|e| print_and_ret!(StatusCode::CREATED, "Post created at {created_at} returned no id: {e}"),
					|i| {
						// inval_all_for_post(i, &inval);
						(StatusCode::OK, i.to_string())
					}
				)
		)
}

pub async fn edit_post(
	session: Session,
	mut tx: Tx<Postgres>,
	// inval: Invalidator,
	Path(id): Path<i32>,
	Json(payload): Json<PostReq>
) -> (StatusCode, String) {
	_ = check_auth!(session);

	let details = post_details(payload);

	if details.content.is_empty() || details.title.is_empty() {
		return (StatusCode::BAD_REQUEST, "The title or content of the post are now empty".into())
	}

	let reading_time = details.reading_time() as i32;
	println!("Trying to edit post with id {id}");

	query("UPDATE posts SET html = $1, orig_markdown = $2, title = $3, tags = $4, reading_time = $5, draft = $6 WHERE id = $7")
		.bind(details.html)
		.bind(details.content)
		.bind(details.title)
		.bind(details.tags)
		.bind(reading_time)
		.bind(details.draft)
		.bind(id)
		.execute(&mut tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Couldn't update/edit post with id {id}: {e:?}"),
			|_| {
				// inval_all_for_post(id, &inval);
				(StatusCode::OK, "OK".into())
			}
		)
}

struct SqlPostDetails {
	content: String,
	html: String,
	title: String,
	tags: String,
	draft: bool
}

impl SqlPostDetails {
	fn reading_time(&self) -> usize {
		// in minutes
		self.content.split_whitespace().count() / 220
	}
}

// Returns an err string or (Text, HTML, Title, Tags)
fn post_details(payload: PostReq) -> SqlPostDetails {
	let PostReq { content, title, tags, draft } = payload;
	let html = shared_data::md_to_html(&content);
	SqlPostDetails { content, html, title, draft, tags: tags.join(",") }
}

fn inval_all_for_post(id: i32, inval: &Invalidator) {
	let post_page = format!("/post/{id}");
	inval.invalidate_all_with_pred(|(_, uri)| {
		let path = uri.path();
		path.starts_with("/home") ||
			path.starts_with("/page") ||
			path == post_page ||
			path == "/sitemap.xml" ||
			path == "/index.xml"
	});
}
