use axum::{
	routing::{get, post},
	extract::{Path, Query, Multipart},
	error_handling::HandleErrorLayer,
	http::StatusCode,
	Router,
	Server,
	Json
};
use axum_auth::AuthBasic;
use axum_sessions::{
	async_session::Session,
	async_session::MemoryStore,
	extractors::{WritableSession, ReadableSession},
	SessionLayer,
};
use axum_sqlx_tx::Tx;
use pulldown_cmark as md;
use rand::Rng;
use serde::Deserialize;
use shared_data::{
	Post,
	PostReq,
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
	ops::Deref,
	time::{SystemTime, UNIX_EPOCH}
};
use tower::ServiceBuilder;

macro_rules! print_and_ret{
	($err: expr, $ret_str: expr) => {{
		eprintln!($ret_str);
		return ($err, format!($ret_str));
	}};
	($ret_str:expr) => {
		print_and_ret!(StatusCode::INTERNAL_SERVER_ERROR, $ret_str)
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
		reading_time INT NOT NULL
	);").execute(&pool)
		.await?;

	println!("Set up posts table in DB...");

	query("CREATE TABLE IF NOT EXISTS users (
		id serial PRIMARY KEY,
		username text NOT NULL,
		hashed_pass text NOT NULL,
		can_post bool NOT NULL
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
	mut tx: Tx<Postgres>,
	Query(PostListParams { count, offset }): Query<PostListParams>
) -> Result<Json<Vec<Post>>, (StatusCode, String)> {
	query_as::<_, Post>(&format!("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, u.username \
		FROM \
		posts p LEFT JOIN users u ON u.id = p.created_by_user \
		ORDER BY id DESC \
		LIMIT {count} \
		OFFSET {offset} \
	;")).fetch_all(&mut *tx)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't retrieve posts {offset},{count}: {e:?}");
			(StatusCode::BAD_REQUEST, format!("Could not retrieve posts: {e:?}"))
		})
}

async fn get_post(mut tx: Tx<Postgres>, Path(id): Path<i32>) -> Result<Json<Post>, (StatusCode, String)> {
	query_as::<_, Post>("SELECT \
		p.id, p.created_at, p.title, p.html, p.orig_markdown, p.tags, p.reading_time, u.username \
		FROM \
		posts p LEFT JOIN users u ON u.id = p.created_by_user \
		WHERE p.id = $1 \
	;").bind(id)
		.fetch_one(&mut *tx)
		.await
		.map(Json)
		.map_err(|e| {
			eprintln!("Couldn't get post {id}: {e:?}");
			(StatusCode::NOT_FOUND, format!("Not found: {e:?}"))
		})
}

// For some reason, this exact order of parameters is necessary to get this to impl `Handler<...>`
// so that it can be used in axum. Don't change them.
pub async fn submit_post(
	session: ReadableSession,
	mut tx: Tx<Postgres>,
	Json(payload): Json<PostReq>
) -> (StatusCode, String) {
	let username = match check_authentication(&*session) {
		Err(reason) => return (StatusCode::UNAUTHORIZED, reason),
		Ok(user) => user
	};

	println!("New post being submitted by user {username}");

	let (content, html_content, title, tags) = post_details(payload);

	if content.is_empty() || title.is_empty() {
		return (StatusCode::BAD_REQUEST, "The title or content of the post are empty".into());
	}

	// Because the UNIX_EPOCH is inherently UTC, the timestamp is for UTC
	// I know this conversion (`as i64`) is not safe, technically, but I doubt this site
	// will survive long enough that we'll get a timestamp higher than i64::MAX
	let Ok(created_at) = SystemTime::now().duration_since(UNIX_EPOCH).map(|c| c.as_secs() as i64) else {
		return (StatusCode::INTERNAL_SERVER_ERROR, "Time has gone backwards?? what the fuck".into())
	};

	// we're just assuming the average wpm for these articles is 220
	let minutes = content.split_whitespace().count() / 220;

	query("INSERT INTO posts
		(created_by_user, created_at, title, html, orig_markdown, tags, reading_time)
		SELECT id, $1, $2, $3, $4, $5, $6 FROM users WHERE username = $7
		RETURNING id
	;").bind(created_at)
		.bind(title)
		.bind(html_content)
		.bind(content)
		.bind(tags)
		.bind(minutes as i32)
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
	if let Err(reason) = check_authentication(&*session) {
		return (StatusCode::UNAUTHORIZED, reason);
	};

	let (content, html_content, title, tags) = post_details(payload);

	if content.is_empty() || title.is_empty() {
		return (StatusCode::BAD_REQUEST, "The title or content of the post are now empty".into())
	}

	println!("Trying to edit post with id {id}");

	query("UPDATE posts SET html = $1, orig_markdown = $2, title = $3, tags = $4 WHERE id = $5")
		.bind(html_content)
		.bind(content)
		.bind(title)
		.bind(tags)
		.bind(id)
		.execute(&mut *tx)
		.await
		.map_or_else(
			|e| print_and_ret!("Couldn't update/edit post with id {id}: {e:?}"),
			|_| (StatusCode::OK, "OK".into())
		)
}

// Returns an err string or (Text, HTML, Title, Tags)
fn post_details(payload: PostReq) -> (String, String, String, String) {
	let PostReq { content, title, tags } = payload;
	let parser = md::Parser::new_ext(&content, md::Options::all());
	let mut html_content = String::new();
	md::html::push_html(&mut html_content, parser);

	(content, html_content, title, tags.join(","))
}

async fn upload_image(session: ReadableSession, mut form: Multipart) -> (StatusCode, String) {
	if let Err(reason) = check_authentication(&*session) {
		return (StatusCode::UNAUTHORIZED, reason);
	};

	// We need to loop over each field of the form
	loop {
		match form.next_field().await {
			Ok(field_opt) => {
				// If it doesn't exist, we've exhausted all the fields
				let Some(field) = field_opt else {
					break;
				};

				if field.name() != Some("file") {
					continue;
				}

				// Just use the current time as the name of the file. Ideally we'd like sha256 hash
				// the data or whatever, but I'm too lazy to do that.
				let file_name = match SystemTime::now().duration_since(UNIX_EPOCH) {
					Ok(t) => t.as_nanos().to_string(),
					Err(e) => print_and_ret!("Couldn't get current time: {e:?}"),
				};

				// And then make sure we can actually get the data of the file
				let image_data = match field.bytes().await {
					Ok(b) if !b.is_empty() => b,
					Err(e) => print_and_ret!("Couldn't get form['file'] data: {e:?}"),
					_ => print_and_ret!(StatusCode::BAD_REQUEST, "Sent an empty image")
				};

				// And create the file to save it at
				let image_dir = match dotenv::var("IMAGE_DIR") {
					Ok(d) => d,
					Err(e) => print_and_ret!("Couldn't get IMAGE_DIR: {e:?}"),
				};

				let image_path = std::path::Path::new(&image_dir);
				let save_path = image_path.join(&file_name);

				return std::fs::write(&save_path, image_data)
					.map_or_else(
						|e| print_and_ret!("Couldn't save the image to {save_path:?}: {e:?}"),
						|_| (StatusCode::OK, file_name)
					);
			},
			Err(err) => print_and_ret!("Couldn't get all fields of request: {err:?}")
		}
	}

	(StatusCode::BAD_REQUEST, "Form didn't contain the requisite 'file' field".into())
}

pub async fn get_image(Path(image): Path<String>) -> (StatusCode, Vec<u8>) {
	// Make sure we know the parent directory
	let image_dir = match dotenv::var("IMAGE_DIR") {
		Ok(d) => d,
		Err(e) => {
			eprintln!("Couldn't get IMAGE_DIR when getting image {image}: {e:?}");
			return (StatusCode::INTERNAL_SERVER_ERROR, vec![])
		}
	};

	// And make sure we can get a full path out of the string they gave us
	let full_path = match std::path::Path::new(&image_dir).join(&image).canonicalize() {
		Ok(p) => p,
		Err(e) => {
			eprintln!("Couldn't canonicalize full path for {image}: {e:?}");
			return (StatusCode::INTERNAL_SERVER_ERROR, vec![])
		}
	};

	// And if the full path isn't still inside the IMAGE_DIR directory, that means they're
	// attempting directory traversal, so we shouldn't let the request continue.
	if !full_path.starts_with(&image_dir) {
		eprintln!("Directory traversal attempted (submitted '{image}', resolved to {full_path:?})");
		return (StatusCode::BAD_REQUEST, vec![]);
	}

	// And then read the file and return information based on what we read
	std::fs::read(&full_path)
		.map_or_else(
			|e| {
				eprintln!("Can't read file at {full_path:?}: {e:?}");
				match e.kind() {
					// If it can't be found, we're just assuming they submitted a bad request,
					// since there shouldn't be any images referenced on the site that don't exist
					// on the fs somewhere
					std::io::ErrorKind::NotFound => (StatusCode::BAD_REQUEST, vec![]),
					_ => (StatusCode::INTERNAL_SERVER_ERROR, vec![])
				}
			},
			|d| (StatusCode::OK, d)
		)
}

pub async fn login(
	mut tx: Tx<Postgres>,
	AuthBasic((username, password)): AuthBasic,
	mut session: WritableSession
) -> (StatusCode, String) {
	println!("User trying to login with session {}", session.id());

	// Just in case they've already logged in
	if check_authentication(&*session).is_ok() {
		return (StatusCode::OK, String::new());
	};

	let Some(pass) = password else {
		return (StatusCode::PRECONDITION_FAILED, "Please include a password".into())
	};

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

// Result<Username, UnauthenticatedReason>
pub fn check_authentication<S: Deref<Target = Session>>(session: &S) -> Result<String, String> {
	session.get(USERNAME_KEY)
		.ok_or_else(|| "User did not log in (/api/login) first".into())
}
