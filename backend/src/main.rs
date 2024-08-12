#![feature(if_let_guard)]

use argon2::{
	password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
	Argon2
};
use axum::{
	extract::{DefaultBodyLimit, Request},
	routing::{get, post},
	Router
};
use const_format::concatcp;
use leptos::prelude::*;
use tower_http::services::ServeDir;
use tower_no_ai::NoAiLayer;
use tower_sessions::{
	MemoryStore,
	SessionManagerLayer
};
use axum_sqlx_tx::Tx;
use images::upload_asset;
use shared_data::Post;
use sqlx::{
	query,
	Postgres,
	postgres::PgPoolOptions,
	PgPool
};
use tokio::net::TcpListener;
use leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes};
use wedding::{
	app::{RouterApp, wedding_app},
	faq::wedding_faq,
	server::{GUESTS_TABLE, RECIPS_TABLE, AxumState}
};

mod images;
mod home;
mod post_list;
mod post;
mod robots;
mod fonts;
mod wedding;
mod auth;
mod blog_api;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	macro_rules! dotenv_num{
		($key:expr, $default:expr, $type:ident) => {
			dotenv::var($key).ok()
				.and_then(|v| v.parse::<$type>().ok())
				.unwrap_or($default)
		}
	}

	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.init();

	// We want to read this one first so that there's the least amount of time between this being
	// loaded into memory and being cleared from memory
	let password = dotenv::var("BASE_PASSWORD");
	// Reset it so nobody else can somehow read it from the env
	std::env::set_var("BASE_PASSWORD", "");
	let username = dotenv::var("BASE_USERNAME");

	// let backend_port = dotenv_num!("BACKEND_PORT", 444, u16);
	let num_connections = dotenv_num!("DB_CONNECTIONS", 80, u32);

	let Ok(db_url) = dotenv::var("DATABASE_URL") else {
		return Err("DATABASE_URL is not set in .env (or is not valid unicode), and is necessary to connect to postgres. Please set it and retry.".into());
	};

	// Verifying that ASSET_DIR is a valid directory and is not readonly
	let Some(asset_dir) = dotenv::var("ASSET_DIR").ok().into_iter().find(|d| !d.is_empty()) else {
		return Err("ASSET_DIR var is not set in .env, and it is necessary to determine where to place assets uploaded as part of posts. Please set it and retry.".into());
	};

	let permissions = match std::fs::metadata(&asset_dir) {
		Ok(mtd) => mtd.permissions(),
		Err(err) => {
			eprintln!("ASSET_DIR does not point to a valid directory: {err:?}");
			return Err(err.into())
		}
	};

	if permissions.readonly() {
		return Err("The directory at ASSET_DIR is readonly; this will prevent assets from being uploaded. Please fix before running the server.".into());
	}

	println!("Storing assets to/Reading assets from {asset_dir}");
	println!("Read .env...");

	let pool = PgPoolOptions::new()
		.max_connections(num_connections)
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

	create_wedding_tables(&pool).await?;

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
					ON CONFLICT (username) DO UPDATE
					SET hashed_pass = EXCLUDED.hashed_pass
					WHERE users.username = EXCLUDED.username
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

	let routes = generate_route_list(RouterApp);

	let leptos_config = get_configuration(None)?;
	// let leptos_config = get_configuration(None).await?;
	let leptos_opts = leptos_config.leptos_options;
	let addr = leptos_opts.site_addr;
	let pkg_dir = format!("{}/{}", leptos_opts.site_root, leptos_opts.site_pkg_dir);
	println!("Packages at {pkg_dir} served at /pkg");

	let state = AxumState { leptos_opts, tx_state };

	let app = Router::<AxumState>::new()
		.route("/", get(home::get_home_view))
		.route("/sitemap.xml", get(robots::get_sitemap_xml))
		.route("/index.xml", get(robots::get_rss_xml))
		.route("/robots.txt", get(robots::get_robots_txt))
		.route("/page/:id", get(home::get_page_view))
		.route("/post/:id", get(post::get_post_view))
		.route("/font/:id", get(fonts::get_font))
		.route("/licenses", get(fonts::get_license_page))
		.route("/api/post/:id", get(blog_api::get_post_json))
		.route("/api/posts", get(blog_api::get_post_list_json))
		.route("/api/new_post", post(blog_api::submit_post))
		.route("/api/edit_post/:id", post(blog_api::edit_post))
		.route("/api/post_asset", post(upload_asset))
		.route("/api/login", get(auth::login))
		.route("/wedding_api/*fn_name", {
			let state = state.clone();
			post(|req: Request| handle_server_fns_with_context(
				move || provide_context(state.clone()),
				req
			))
		})
		.route("/wedding/faq", get(wedding_faq))
		.nest("/wedding", Router::new()
			.leptos_routes_with_context(
				&state,
				routes,
				{
					let state = state.clone();
					move || provide_context(state.clone())
				},
				{
					let state = state.clone();
					move || wedding_app(state.clone())
				}
			)
		)
		.nest_service("/api/assets/", ServeDir::new(asset_dir))
		.nest_service("/pkg/", ServeDir::new(pkg_dir))
		// I want to be able to upload 10mb assets if I so please.
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024))
		.layer(SessionManagerLayer::new(session_store))
		.layer(tx_layer)
		.layer(NoAiLayer::new("https://fsn1-speed.hetzner.com/10GB.bin"))
		.with_state(state);

	println!("Serving axum at http://{addr}...");

	let listener = TcpListener::bind(addr).await?;
	axum::serve(listener, app).await.unwrap();

	Ok(())
}

pub async fn create_wedding_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
	query(concatcp!("CREATE TABLE IF NOT EXISTS ", GUESTS_TABLE, "(
		id uuid PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
		name text NOT NULL,
		party_size INT NOT NULL,
		full_address text,
		email text,
		extra_notes text
	);")).execute(pool)
		.await?;

	query(concatcp!("CREATE TABLE IF NOT EXISTS ", RECIPS_TABLE, "(
		id serial PRIMARY KEY,
		name text NOT NULL,
		address text,
		email text
	);")).execute(pool)
		.await?;

	Ok(())
}
