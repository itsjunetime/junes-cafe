use argon2::{
	password_hash::{PasswordHash, PasswordVerifier},
	Argon2
};
use serde::{Serialize, Deserialize};
use sqlx::{query, Postgres, Row};
use axum_sqlx_tx::Tx;
use tower_sessions::Session;
use axum::{response::Redirect, Form};

pub const USERNAME_KEY: &str = "authenticated_username";

pub async fn get_username(session: &Session) -> Option<String> {
	session.get::<String>(crate::auth::USERNAME_KEY).await.ok().flatten()
}

#[macro_export]
macro_rules! check_auth{
	($session:ident) => {
		match $crate::auth::get_username(&$session).await {
			Some(user) => user,
			None => return (StatusCode::UNAUTHORIZED, "User did not login (/apiv2/login) first".to_string())
		}
	};
}

#[derive(Deserialize, Serialize)]
pub struct LoginQuery {
	pub redir_to: Option<String>,
	pub err_msg: Option<String>
}

#[derive(Deserialize)]
pub struct LoginParams {
	username: String,
	password: String,
	redir_to: Option<String>
}

pub async fn login(
	mut tx: Tx<Postgres>,
	session: Session,
	Form(LoginParams { username, password, redir_to }): Form<LoginParams>,
) -> Redirect {
	let err = |err_msg: &'static str| -> Redirect {
		Redirect::to(
			&format!(
				"/login?{}",
				serde_urlencoded::to_string(
					LoginQuery {
						redir_to: redir_to.clone(),
						err_msg: Some(err_msg.to_string())
					}
				).unwrap()
			)
		)
	};
	let all_good = || Redirect::to(redir_to.as_ref().map_or_else(|| "/admin", String::as_str));

	// Just in case they've already logged in
	if get_username(&session).await.is_some() {
		return all_good();
	};

	let session_id = session.id();

	// Only get the pass if it's not empty
	if password.is_empty() {
		return err("Please include a password");
	};

	if username.is_empty() {
		eprintln!("Session {session_id:?} sent a login request with an empty username");
		return err("Please include a username");
	}

	println!("User trying to login with session {session_id:?} and username {username}");

	let Ok(hash) = query("SELECT hashed_pass FROM users WHERE username = $1")
		.bind(&username)
		.fetch_one(&mut tx)
		.await
		.and_then(|row| row.try_get::<String, _>("hashed_pass"))
		.inspect_err(|e| {
			eprintln!("Database error when logging in: {e:?}");
		}) else {
			// It would make more sense to send an INTERNAL_SERVER_ERROR but that could expose a
			// vulnerability if they were able to reliably cause a database error with a certain
			// input, so we are just logging the error then giving them a generic response
			return err("Incorrect username or password");
		};

	let Ok(hash_struct) = PasswordHash::new(&hash)
		.inspect_err(|e| {
			eprintln!("Couldn't create password hash from hash in database ({e:?}); has anyone messed with your db?");
		}) else {
			return err("Incorrect username or password");
		};

	match Argon2::default().verify_password(password.as_bytes(), &hash_struct) {
		Ok(()) => {
			println!("Trying to log in {username} with session_id {:?}", session.id());

			if let Err(e) = session.insert(USERNAME_KEY, username).await {
				println!("Could not save session: {e}");
				return err("Failed to save session; unable to log you in");
			}

			all_good()
		},
		Err(e) => {
			if e == argon2::password_hash::Error::Password {
				println!("Given password is incorrect (ugh)");
			} else {
				println!("Password verification failed with error {e}");
			}

			err("Incorrect username or password")
		}
	}
}
