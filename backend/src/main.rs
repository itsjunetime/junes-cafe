use argon2::{
	password_hash::{
		rand_core::OsRng,
		PasswordHash, PasswordHasher, PasswordVerifier, SaltString
	},
	Argon2
};
use axum::{
	routing::{get, post},
	extract::{Path, DefaultBodyLimit, Query},
	http::StatusCode,
	Router,
	Json
};
use axum_auth::AuthBasic;
use tower_sessions::{
	MemoryStore,
	Session,
	SessionManagerLayer
};
use serde::Deserialize;
use axum_sqlx_tx::Tx;
use images::{upload_asset, get_asset};
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
	},
};
use std::{
	net::SocketAddr,
	time::{SystemTime, UNIX_EPOCH}
};
use tokio::net::TcpListener;

mod images;
mod home;
mod post_list;
mod post;
mod robots;
mod md_to_html;

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
			Some(user) => user,
			None => return (StatusCode::UNAUTHORIZED, "User did not login (/api/login) first".to_string())
		}
	};
	($session:ident, noret) => {
		$session.get::<String>($crate::USERNAME_KEY).await.ok().flatten()
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

	// We want to read this one first so that there's the least amount of time between this being
	// loaded into memory and being cleared from memory
	let password = dotenv::var("BASE_PASSWORD");
	// Reset it so nobody else can somehow read it from the env
	std::env::set_var("BASE_PASSWORD", "");
	let username = dotenv::var("BASE_USERNAME");

	let backend_port = dotenv_num!("BACKEND_PORT", 444, u16);
	let num_connections = dotenv_num!("DB_CONNECTIONS", 80, u32);

	let db_url = match dotenv::var("DATABASE_URL") {
		Ok(url) => url,
		Err(_) => {
			return Err("DATABASE_URL is not set in .env (or is not valid unicode), and is necessary to connect to postgres. Please set it and retry.".into());
		}
	};

	// Verifying that ASSET_DIR is a valid directory and is not readonly
	let Some(dir) = dotenv::var("ASSET_DIR").ok().and_then(|d| (!d.is_empty()).then_some(d)) else {
		return Err("ASSET_DIR var is not set in .env, and it is necessary to determine where to place assets uploaded as part of posts. Please set it and retry.".into());
	};

	let permissions = match std::fs::metadata(&dir) {
		Ok(mtd) => mtd.permissions(),
		Err(err) => {
			eprintln!("ASSET_DIR does not point to a valid directory: {err:?}");
			return Err(err.into())
		}
	};

	if permissions.readonly() {
		return Err("The directory at ASSET_DIR is readonly; this will prevent assets from being uploaded. Please fix before running the server.".into());
	}

	println!("Storing assets to/Reading assets from {dir}");
	println!("Read .env...");

	let pool = PgPoolOptions::new()
		.max_connections(num_connections)
		// We need the `barista` database to exist before we start
		.connect(&db_url)
		.await?;

	println!("Connected to postgres...");

	// Make sure the table that we're working on exists
	// This doesn't verify that it exists with these exact datatypes in each column, which would
	// be ideal, but I can't find a way to easily do that so I'm not going to for now
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

	// We just assume that if you have an account, you can post. Maybe we can add more
	// fine-grained controls later. Doesn't really matter for now tho.
	query("CREATE TABLE IF NOT EXISTS users (
		id serial PRIMARY KEY,
		username text NOT NULL UNIQUE,
		hashed_pass text NOT NULL
	);").execute(&pool)
		.await?;

	match (username, password) {
		(Ok(name), Ok(pass)) => {
			println!("Adding user {name} to the db if not already exists (else updating password)");

			let salt = SaltString::generate(&mut OsRng);
			let argon = Argon2::default();

			let hash = match argon.hash_password(pass.as_bytes(), &salt) {
				Ok(hash) => hash.to_string(),
				Err(err) => {
					eprintln!("Couldn't hash the given password at .env:BASE_PASSWORD: {err:?}");
					return Err(err.into());
				}
			};

			// We insert the user into the db, but if there's a conflict (if the username already
			// exists), then we just leave it be, since the hash for the same password can change.
			// If you want to change the password, you'll have to manually go into the database and
			// clear the user.
			query("INSERT INTO users (username, hashed_pass)
				  VALUES ($1, $2)
				  ON CONFLICT (username) DO NOTHING
			;").bind(name)
				.bind(hash)
				.execute(&pool)
				.await?;
		},
		(Err(_), Err(_)) => println!("No base user specified; adding more users will be difficult"),
		_ => {
			return Err("Either a base username or password was specified, but not the other. Cannot proceed; please specify both or neither.".into());
		}
	};

	// Set up sessions for authentication
	// And if we have to restart the server, then we're ok with losing sessions, so do an only
	// in-memory session store, just for simplicity.
	let session_store = MemoryStore::default();

	let (tx_state, tx_layer) = Tx::<Postgres>::setup(pool);

	let app = Router::new()
		.route("/", get(home::get_home_view))
		.route("/sitemap.xml", get(robots::get_sitemap_xml))
		.route("/index.xml", get(robots::get_rss_xml))
		.route("/page/:id", get(home::get_page_view))
		.route("/post/:id", get(post::get_post_view))
		.route("/api/post/:id", get(get_post_json))
		.route("/api/posts", get(get_post_list_json))
		.route("/api/new_post", post(submit_post))
		.route("/api/edit_post/:id", post(edit_post))
		.route("/api/post_asset", post(upload_asset))
		.route("/api/assets/:id", get(get_asset))
		.route("/api/login", get(login))
		// I want to be able to upload 10mb assets if I so please.
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024))
		.layer(SessionManagerLayer::new(session_store))
		.layer(tx_layer)
		.with_state(tx_state);

	let addr = SocketAddr::from(([127, 0, 0, 1], backend_port));

	println!("Serving axum at {addr}...");

	let listener = TcpListener::bind(addr).await?;
	axum::serve(listener, app).await.unwrap();

	Ok(())
}

async fn get_post_list(
	session: &Session,
	tx: &mut Tx<Postgres>,
	count: u32,
	offset: u32
) -> Result<Vec<Post>, sqlx::Error> {
	// If the user is logged in, then they can see all draft posts as well.
	let draft_clause = if check_auth!(session, noret).is_some() {
		""
	} else {
		"WHERE p.draft IS NOT TRUE"
	};

	query_as::<_, Post>(&format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, p.draft, u.username \
		FROM posts p \
		LEFT JOIN users u ON u.id = p.created_by_user \
		{draft_clause} \
		ORDER BY id DESC \
		LIMIT {count} \
		OFFSET {offset} \
	;")).fetch_all(tx)
		.await
}

#[derive(Deserialize)]
struct PostListParams {
	count: u32,
	offset: u32
}

async fn get_post_list_json(
	session: Session,
	mut tx: Tx<Postgres>,
	Query(PostListParams { count, offset }): Query<PostListParams>
) -> Result<Json<Vec<Post>>, (StatusCode, String)> {
	get_post_list(&session, &mut tx, count, offset)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't retrieve posts {offset},{count}: {e:?}");
			match e {
				sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, format!("The specified offset,limit of {offset},{count} corresponds to no posts")),
				_ => (StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't retrieve posts: {e:?}"))
			}
		})
}

async fn get_post(
	session: Session,
	mut tx: Tx<Postgres>,
	Path(id): Path<i32>
) -> Result<Post, sqlx::Error> {
	// If they're logged in, they should be able to view drafts
	let where_clause = if check_auth!(session, noret).is_some() {
		"WHERE p.id = $1"
	} else {
		"WHERE (p.id = $1 AND p.draft IS NOT TRUE)"
	};

	let query_str = format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, p.draft, u.username \
		FROM \
		posts p LEFT JOIN users u ON u.id = p.created_by_user \
		{where_clause}\
	;");

	query_as::<_, Post>(&query_str)
		.bind(id)
		.fetch_one(&mut tx)
		.await
}

async fn get_post_json(
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
	let minutes = details.content.split_whitespace().count() / 220;

	let ret = query("INSERT INTO posts
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
			|e| print_and_ret!("Failed to create new post: {e:?}"),
			|r| r.try_get::<i32, _>("id")
				.map_or_else(
					|e| print_and_ret!(StatusCode::CREATED, "Post created at {created_at} returned no id: {e:?}"),
					|i| (StatusCode::OK, i.to_string())
				)
		);

	if let Err(e) = robots::update_sitemap_xml(&session, &mut tx).await {
		eprintln!("Couldn't update sitemap after submit: {e}");
	}

	if let Err(e) = robots::update_rss_xml(&session, &mut tx).await {
		eprintln!("Couldn't update rss xml after submit: {e}");
	}

	ret
}

pub async fn edit_post(
	session: Session,
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

	let ret = query("UPDATE posts SET html = $1, orig_markdown = $2, title = $3, tags = $4, draft = $5 WHERE id = $6")
		.bind(details.html)
		.bind(details.content)
		.bind(details.title)
		.bind(details.tags)
		.bind(details.draft)
		.bind(id)
		.execute(&mut tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Couldn't update/edit post with id {id}: {e:?}"),
			|_| (StatusCode::OK, "OK".into())
		);

	// The only reason we'd need to udpate the sitemap is if we made a post public
	if details.draft {
		if let Err(e) = robots::update_sitemap_xml(&session, &mut tx).await {
			eprintln!("Couldn't update sitemap after edit: {e}");
		}
	}

	if let Err(e) = robots::update_rss_xml(&session, &mut tx).await {
		eprintln!("Couldn't udpate rss xml after edit: {e}");
	}

	ret
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
	let html = md_to_html::md_to_html(&content);
	SqlPostDetails { content, html, title, draft, tags: tags.join(",") }
}

pub async fn login(
	mut tx: Tx<Postgres>,
	AuthBasic((username, password)): AuthBasic,
	session: Session
) -> Result<(), (StatusCode, &'static str)> {
	// Just in case they've already logged in
	if check_auth!(session, noret).is_some() {
		return Ok(());
	};

	let session_id = session.id();

	// Only get the pass if it's not empty
	let Some(pass) = password.and_then(|p| (!p.is_empty()).then_some(p)) else {
		eprintln!("Session {session_id:?} sent a login request with an empty password");
		return Err((StatusCode::PRECONDITION_FAILED, "Please include a password"));
	};

	if username.is_empty() {
		eprintln!("Session {session_id:?} sent a login request with an empty username");
		return Err((StatusCode::PRECONDITION_FAILED, "Please include a username"));
	}

	println!("User trying to login with session {session_id:?} and username {username}");

	let unauth = || (StatusCode::UNAUTHORIZED, "Incorrect username or password");

	let hash = query("SELECT hashed_pass FROM users WHERE username = $1")
		.bind(&username)
		.fetch_one(&mut tx)
		.await
		.and_then(|row| row.try_get::<String, _>("hashed_pass"))
		.map_err(|e| {
			eprintln!("Database error when logging in: {e:?}");
			// It would make more sense to send an INTERNAL_SERVER_ERROR but that could expose a
			// vulnerability if they were able to reliably cause a database error with a certain
			// input, so we are just logging the error then giving them a generic response
			unauth()
		})?;

	let hash_struct = PasswordHash::new(&hash)
		.map_err(|e| {
			eprintln!("Couldn't create password hash from hash in database ({e:?}); has anyone messed with your db?");
			unauth()
		})?;

	match Argon2::default().verify_password(pass.as_bytes(), &hash_struct) {
		Ok(()) => {
			println!("Trying to log in {username} with session_id {:?}", session.id());

			if let Err(err) = session.insert(USERNAME_KEY, username).await {
				println!("Could not save session: {err}");
				return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to save session; unable to log you in"));
			}

			Ok(())
		},
		Err(e) => {
			if e == argon2::password_hash::Error::Password {
				println!("Given password '{pass}' is incorrect (ugh)");
			} else {
				println!("Password verification failed with error {e}");
			}

			Err(unauth())
		}
	}
}
