use axum::{
	routing::{get, post},
	extract::{Path, Query, DefaultBodyLimit},
	error_handling::HandleErrorLayer,
	http::StatusCode,
	Router,
	Server,
	Json
};
use axum_auth::AuthBasic;
use axum_sessions::{
	async_session::MemoryStore,
	extractors::{WritableSession, ReadableSession},
	SessionLayer,
};
use axum_sqlx_tx::Tx;
use images::{upload_image, get_image};
use pulldown_cmark as md;
use rand::Rng;
use serde::Deserialize;
use shared_data::{
	Post,
	PostReq,
	sqlx,
	sqlx::{
		query,
		query_as,
		Row,
		Postgres,
		postgres::PgPoolOptions
	}
};
use std::{
	net::SocketAddr,
	time::{SystemTime, UNIX_EPOCH}
};
use tower::ServiceBuilder;

mod images;

#[macro_export]
macro_rules! print_and_ret{
	($err: expr, $ret_str: expr) => {{
		eprintln!($ret_str);
		return ($err, format!($ret_str));
	}};
	($ret_str:expr) => {
		print_and_ret!(StatusCode::INTERNAL_SERVER_ERROR, $ret_str)
	}
}

#[macro_export]
macro_rules! check_auth{
	($session:ident) => {
		match check_auth!($session, noret) {
			Ok(user) => user,
			Err(err) => return (StatusCode::UNAUTHORIZED, err)
		}
	};
	($session:ident, noret) => {
		$session.get::<String>(USERNAME_KEY)
			.ok_or_else(|| "User did not log in (/api/login) first".to_string())
	}
}

const USERNAME_KEY: &str = "authenticated_username";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	macro_rules! dotenv_num{
		($key:expr, $default:expr, $type:ident) => {
			dotenv::var($key).ok()
				.and_then(|v| v.parse::<$type>().ok())
				.unwrap_or($default)
		}
	}

	let backend_port = dotenv_num!("BACKEND_PORT", 444, u16);
	let num_connections = dotenv_num!("DB_CONNECTIONS", 80, u32);

	let db_table = dotenv::var("DB_TABLE").unwrap_or_else(|_| "barista".into());
	let db_host = dotenv::var("DB_HOST").unwrap_or_else(|_| "localhost".into());
	let db_user = dotenv::var("DB_USER")?;

	// Verifying that IMAGE_DIR is a valid directory and is not readonly
	let Some(dir) = dotenv::var("IMAGE_DIR").ok().and_then(|d| (!d.is_empty()).then_some(d)) else {
		eprintln!("IMAGE_DIR var is not set in .env, and it is necessary to determine \
				   where to place images uploaded as part of posts. Please set it and retry.");
		return Ok(())
	};

	let permissions = match std::fs::metadata(&dir) {
		Ok(mtd) => mtd.permissions(),
		Err(err) => {
			eprintln!("IMAGE_DIR does not point to a valid directory: {err:?}");
			return Ok(())
		}
	};

	if permissions.readonly() {
		eprintln!("The directory at IMAGE_DIR is readonly; this will prevent images from being uploaded. \
				  Please fix before running the server.");
		return Ok(())
	}

	println!("Storing images to/Reading images from {dir}");
	println!("Read .env...");

	let pool = PgPoolOptions::new()
		.max_connections(num_connections)
		// We need the `barista` table to exist before we start
		.connect(&format!("postgresql://{db_user}@{db_host}/{db_table}"))
		.await?;

	println!("Connected to postgres...");

	// Make sure the table that we're working on exists
	// This doesn't verify that it exists with these exact datatypes in each column, which would be
	// ideal, but I can't find a way to easily do that so I'm not going to for now
	query("CREATE TABLE IF NOT EXISTS posts (
		id serial PRIMARY KEY,
		created_by_user BIGINT NOT NULL,
		created_at BIGINT NOT NULL,
		title text NOT NULL,
		html text NOT NULL,
		orig_markdown text NOT NULL,
		tags text,
		reading_time INT NOT NULL,
		draft bool NOT NULL
	);").execute(&pool)
		.await?;

	println!("Set up posts table in DB...");

	// We just assume that if you have an account, you can post. Maybe we can add more fine-grained
	// controls later. Doesn't really matter for now tho.
	query("CREATE TABLE IF NOT EXISTS users (
		id serial PRIMARY KEY,
		username text NOT NULL,
		hashed_pass text NOT NULL
	);").execute(&pool)
		.await?;

	// Set up sessions for authentication
	// And if we have to restart the server, then we're ok with losing sessions, so do an only
	// in-memory session store, just for simplicity.
	let session_store = MemoryStore::new();
	let mut rng = rand::thread_rng();
	let secret: Vec<u8> = vec![0; 128].into_iter().map(|_| rng.gen()).collect();

	let app = Router::new()
		.route("/api/posts", get(get_post_list))
		.route("/api/post/:id", get(get_post))
		.route("/api/new_post", post(submit_post))
		.route("/api/edit_post/:id", post(edit_post))
		.route("/api/post_image", post(upload_image))
		.route("/api/images/:id", get(get_image))
		.route("/api/login", get(login))
		// I want to be able to upload 10mb images if I so please.
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024))
		.layer(SessionLayer::new(session_store, &secret))
		.layer(
			ServiceBuilder::new()
				.layer(HandleErrorLayer::new(|err| async move {
					eprintln!("Couldn't commit transaction: {err:?}");
					(StatusCode::INTERNAL_SERVER_ERROR, format!("Postgres Transaction failed: {err:?}"))
				}))
				.layer(axum_sqlx_tx::Layer::new(pool))
		);

	let addr = SocketAddr::from(([127, 0, 0, 1], backend_port));

	println!("Serving axum...");

	Server::bind(&addr)
		.serve(app.into_make_service())
		.await
		.unwrap();

	Ok(())
}

#[derive(Deserialize)]
struct PostListParams {
	count: u32,
	offset: u32
}

async fn get_post_list(
	session: ReadableSession,
	mut tx: Tx<Postgres>,
	Query(PostListParams { count, offset }): Query<PostListParams>
) -> Result<Json<Vec<Post>>, (StatusCode, String)> {
	// If the user is logged in, then they can see all draft posts as well.
	let draft_clause = if check_auth!(session, noret).is_ok() {
		""
	} else {
		"WHERE p.draft IS NOT TRUE"
	};

	query_as::<_, Post>(&format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, u.username \
		FROM posts p \
		LEFT JOIN users u ON u.id = p.created_by_user \
		{draft_clause} \
		ORDER BY id DESC \
		LIMIT {count} \
		OFFSET {offset} \
	;")).fetch_all(&mut *tx)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't retrieve posts {offset},{count}: {e:?}");
			match e {
				sqlx::Error::RowNotFound => (StatusCode::BAD_REQUEST, format!("The specified offset,limit of {offset},{count} corresponds to no posts")),
				_ => (StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't retrieve posts: {e:?}"))
			}
		})
}

async fn get_post(
	session: ReadableSession,
	mut tx: Tx<Postgres>,
	Path(id): Path<i32>
) -> Result<Json<Post>, (StatusCode, String)> {
	// If they're logged in, they should be able to view drafts
	let where_clause = if check_auth!(session, noret).is_ok() {
		"WHERE p.id = $1"
	} else {
		"WHERE (p.id = $1 AND p.draft IS NOT TRUE)"
	};

	let query_str = format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, u.username \
		FROM \
		posts p LEFT JOIN users u ON u.id = p.created_by_user \
		{where_clause}\
	;");

	println!("Querying: '{query_str}'");

	query_as::<_, Post>(&query_str).bind(id)
		.fetch_one(&mut *tx)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't get post {id}: {e:?}");
			match e {
				sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Post not found".into()),
				_ => (StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't retrieve post: {e:?}"))
			}
		})
}

pub async fn submit_post(
	session: ReadableSession,
	mut tx: Tx<Postgres>,
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
		return (StatusCode::INTERNAL_SERVER_ERROR, "Time has gone backwards?? what the fuck".into())
	};

	// we're just assuming the average wpm for these articles is 220
	let minutes = details.content.split_whitespace().count() / 220;

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
		.fetch_one(&mut *tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Failed to create new post: {e:?}"),
			|r| r.try_get::<i32, _>("id")
				.map_or_else(
					|e| print_and_ret!(StatusCode::CREATED, "Post created at {created_at} returned no id: {e:?}"),
					|i| (StatusCode::OK, i.to_string())
				)
		)
}

pub async fn edit_post(
	session: ReadableSession,
	mut tx: Tx<Postgres>,
	Path(id): Path<i32>,
	Json(payload): Json<PostReq>
) -> (StatusCode, String) {
	_ = check_auth!(session);

	let details = post_details(payload);

	if details.content.is_empty() || details.title.is_empty() {
		return (StatusCode::BAD_REQUEST, "The title or content of the post are now empty".into())
	}

	println!("Trying to edit post with id {id}");

	query("UPDATE posts SET html = $1, orig_markdown = $2, title = $3, tags = $4, draft = $5 WHERE id = $5")
		.bind(details.html)
		.bind(details.content)
		.bind(details.title)
		.bind(details.tags)
		.bind(details.draft)
		.bind(id)
		.execute(&mut *tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Couldn't update/edit post with id {id}: {e:?}"),
			|_| (StatusCode::OK, "OK".into())
		)
}

struct SqlPostDetails {
	content: String,
	html: String,
	title: String,
	tags: String,
	draft: bool
}

// Returns an err string or (Text, HTML, Title, Tags)
fn post_details(payload: PostReq) -> SqlPostDetails {
	let PostReq { content, title, tags, draft } = payload;
	let parser = md::Parser::new_ext(&content, md::Options::all());
	let mut html = String::new();
	md::html::push_html(&mut html, parser);

	SqlPostDetails { content, html, title, draft, tags: tags.join(",") }
}

pub async fn login(
	mut tx: Tx<Postgres>,
	AuthBasic((username, password)): AuthBasic,
	mut session: WritableSession
) -> (StatusCode, String) {
	// Just in case they've already logged in
	if check_auth!(session, noret).is_ok() {
		return (StatusCode::OK, String::new());
	};

	// Only get the pass if it's not empty
	let Some(pass) = password.and_then(|p| (!p.is_empty()).then_some(p)) else {
		eprintln!("Session {} sent a login request with an empty password", session.id());
		return (StatusCode::PRECONDITION_FAILED, "Please include a password".into())
	};

	if username.is_empty() {
		eprintln!("Session {} sent a login request with an empty username", session.id());
		return (StatusCode::PRECONDITION_FAILED, "Please include a username".into())
	}

	println!("User trying to login with session {} and username {username}", session.id());

	let unauth = || (StatusCode::UNAUTHORIZED, "Incorrect username or password".into());

	query("SELECT hashed_pass FROM users WHERE username = $1")
		.bind(&username)
		.fetch_one(&mut *tx)
		.await
		.and_then(|row| row.try_get::<String, _>("hashed_pass"))
		.map_or_else(
			|e| {
				eprintln!("Database error when logging in: {e:?}");
				unauth()
			},
			|p| argon2::verify_encoded(&p, pass.as_bytes())
				.map_or_else(
					|e| print_and_ret!("Couldn't verify password: {e:?}"),
					|verified| if verified {
						println!("Trying to log in {username} with session_id {}", session.id());

						if let Err(err) = session.insert(USERNAME_KEY, username) {
							print_and_ret!("Could not save session: {err}");
						}

						(StatusCode::OK, String::new())
					} else {
						unauth()
					}
				)
		)
}
