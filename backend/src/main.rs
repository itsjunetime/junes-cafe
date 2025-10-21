#![feature(if_let_guard)]

use core::{net::{Ipv4Addr, SocketAddr, SocketAddrV4}, str::FromStr};

use argon2::{
	password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
	Argon2
};
use axum::{
	extract::DefaultBodyLimit, routing::{get, post}, Router
};
use axum_server::tls_rustls::RustlsConfig;
use http::{Method, StatusCode};
use leptos::prelude::*;
use leptos_axum::handle_server_fns_with_context;
use tower_cache::options::CacheOptions;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_no_ai::NoAiLayer;
use tower_sessions::{
	MemoryStore,
	SessionManagerLayer
};
use axum_sqlx_tx::Tx;
use shared_data::Post;
use sqlx::{
	query,
	Postgres,
	postgres::PgPoolOptions,
};
use backend::AxumState;
use tracing_subscriber::EnvFilter;

mod images;
mod home;
mod post_list;
mod post;
mod robots;
mod fonts;
mod blog_api;
mod pages;

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

struct Config {
	base_password: Option<String>,
	base_username: Option<String>,
	database_url: String,
	asset_dir: String,
	rustls_config: Option<RustlsConfig>,
	db_connections: u32,
	backend_port: Option<u16>
}

async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
	let systemd_creds_dir = std::env::var("CREDENTIALS_DIRECTORY");

	let base_username = dotenv::var("BASE_USERNAME").ok();
	let (base_password, db_url) = match systemd_creds_dir {
		Ok(dir) => {
			let (a, b) = tokio::join!(
				tokio::fs::read_to_string(format!("{dir}/base_password")),
				tokio::fs::read_to_string(dir + "/database_url")
			);

			(a.ok(), b.ok())
		}
		Err(_) => {
			let (a, b) = (
				dotenv::var("BASE_PASSWORD"),
				dotenv::var("DATABASE_URL")
			);

			(a.ok(), b.ok())
		}
	};

	let Some(database_url) = db_url else {
		return Err("DATABASE_URL is not set in .env (or is not valid unicode), and is necessary to connect to postgres. Please set it and retry.".into());
	};

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

	let cert_file = dotenv::var("CERT_FILE");
	let key_file = dotenv::var("KEY_FILE");

	let rustls_config = match (cert_file, key_file) {
		(Ok(_), Err(_)) | (Err(_), Ok(_)) =>
			return Err("You have to set either BOTH `CERT_FILE` and `KEY_FILE` or neither".into()),
		(Ok(c_f), Ok(k_f)) => {
			println!("Creating server config with cert file {c_f:?} and key file {k_f:?}");
			Some(RustlsConfig::from_pem_file(c_f, k_f)
				.await
				.inspect_err(|e| eprintln!("Couldn't make rustls config: {e}"))?)
		},
		(Err(_), Err(_)) => {
			println!("Creating server config without tls");
			None
		}
	};

	let db_connections = dotenv::var("DB_CONNECTIONS")
		.ok()
		.and_then(|n| n.parse()
			.inspect_err(|e| println!("Can't parse value for DB_CONNECTIONS ({n:?}) to u16: {e}"))
			.ok()
		)
		.unwrap_or(80);

	let backend_port = dotenv::var("BACKEND_PORT")
		.ok()
		.and_then(|p| <u16 as FromStr>::from_str(&p)
			.inspect_err(|e| println!("Couldn't convert BACKEND_PORT {p:?} to a u16: {e}"))
			.ok()
		);

	Ok(Config {
		base_password,
		base_username,
		database_url,
		asset_dir,
		rustls_config,
		backend_port,
		db_connections
	})
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	let config = load_config().await?;

	println!("Trying to connect to postgres...");

	let pool = PgPoolOptions::new()
		.max_connections(config.db_connections)
		.connect(&config.database_url)
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

	match (config.base_username, config.base_password) {
		(Some(name), Some(pass)) => {
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
		(None, None) => println!("No base user specified; adding more users will be difficult"),
		_ => {
			return Err("Either a base username or password was specified, but not the other. Cannot proceed; please specify both or neither.".into());
		}
	};

	// Set up sessions for authentication
	// And if we have to restart the server, then we're ok with losing sessions, so do an only
	// in-memory session store, just for simplicity.
	let session_store = MemoryStore::default();

	let (tx_state, tx_layer) = Tx::<Postgres>::setup(pool);

	let leptos_config = get_configuration(None)?;
	let mut leptos_opts = leptos_config.leptos_options;

	if let Some(port) = config.backend_port {
		leptos_opts.site_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
	}

	let addr = leptos_opts.site_addr;
	let pkg_dir = format!("{}/{}", leptos_opts.site_root, leptos_opts.site_pkg_dir);
	println!("Packages at {pkg_dir} served at /pkg");

	let cache_options = CacheOptions::new(
		Some(StatusCode::OK..StatusCode::INTERNAL_SERVER_ERROR),
		Some(Method::GET)
	);

	let invalidator = cache_options.invalidator();

	any_spawner::Executor::init_tokio()?;

	let state = AxumState { leptos_opts, tx_state, invalidator };
	let server_fn_state = state.clone();

	let app = Router::<AxumState>::new()
		.route("/sitemap.xml", get(robots::get_sitemap_xml))
		.route("/index.xml", get(robots::get_rss_xml))
		.route("/robots.txt", get(robots::get_robots_txt))
		.route("/licenses", get(fonts::get_license_page))
		// hmmmmmmm... caching... how do we insert 'sessions' as dependencies... who knows...
		// We're putting this layer right here for now so that it only applies to the routes added
		// before it is called. Those are the things that, at least at the moment, shouldn't really
		// change regardless of logged-in status or not.
		// .layer(CacheLayer::<SendLinearMap<_, _>, _, _>::new(cache_options))
		.route("/", get(home::get_home_view))
		.route("/post/{id}", get(post::get_post_view))
		.route("/page/{id}", get(home::get_page_view))
		.route("/font/{id}", get(fonts::get_font))
		.route("/login", get(pages::login::login_html))
		.route("/api/login", post(backend::auth::login))
		.nest_service("/api/assets/", ServeDir::new(config.asset_dir))
		.route("/api/{fn_name}", post(move |req| handle_server_fns_with_context(
			move || provide_context(server_fn_state.clone()),
			req
		)))
		.route("/admin/new_post", get(pages::edit_post::new_post))
		.route("/admin/edit_post/{id}", get(pages::edit_post::edit_post_handler))
		.route("/admin", get(pages::admin::admin))
		.nest_service("/pkg/", ServeDir::new(pkg_dir))
		// I want to be able to upload 10mb assets if I so please.
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024))
		.layer(SessionManagerLayer::new(session_store))
		.layer(tx_layer)
		.layer(TraceLayer::new_for_http())
		.layer(NoAiLayer::new("https://fsn1-speed.hetzner.com/10GB.bin"))
		.with_state(state);

	println!("Serving axum at https://{addr}...");

	if let Some(rustls_config) = config.rustls_config {
		axum_server::bind_rustls(addr, rustls_config)
			.serve(app.into_make_service())
			.await?;
	} else {
		axum_server::bind(addr)
			.serve(app.into_make_service())
			.await?;
	}

	Ok(())
}
