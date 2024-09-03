use http::StatusCode;
use axum_auth::AuthBasic;
use argon2::{
	password_hash::{PasswordHash, PasswordVerifier},
	Argon2
};
use sqlx::{Postgres, query, Row};
use axum_sqlx_tx::Tx;
use tower_sessions::Session;

pub const USERNAME_KEY: &str = "authenticated_username";

#[macro_export]
macro_rules! check_auth{
	($session:ident) => {
		match check_auth!($session, noret) {
			Some(user) => user,
			None => return (StatusCode::UNAUTHORIZED, "User did not login (/api/login) first".to_string())
		}
	};
	($session:ident, noret) => {
		$session.get::<String>($crate::auth::USERNAME_KEY).await.ok().flatten()
	}
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
