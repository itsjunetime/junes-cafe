use std::{borrow::Cow, str::FromStr};

#[cfg(not(target_family = "wasm"))]
pub mod login;
#[cfg(not(target_family = "wasm"))]
pub mod admin;
pub mod edit_post;

pub enum HtmlOrRedirect<H> {
	Html(H),
	Redirect(Redirect)
}

pub struct Redirect {
	force_login: bool,
	redir_to: RedirLocation
}

#[derive(Default)]
pub enum RedirLocation {
	#[default]
	Admin,
	EditPost(u32),
	NewPost
}

impl Redirect {
	fn url(&self) -> Cow<'static, str> {
		let mut eventual_dest: Cow<str> = match self.redir_to {
			RedirLocation::Admin => "/admin".into(),
			RedirLocation::NewPost => "/admin/new_post".into(),
			RedirLocation::EditPost(id) => format!("/admin/edit_post/{id}").into(),
		};

		if self.force_login {
			eventual_dest = format!("/login?redir_to={eventual_dest}").into();
		}

		eventual_dest
	}
}

impl FromStr for RedirLocation {
	type Err = ();
	fn from_str(mut s: &str) -> Result<Self, Self::Err> {
		const EDIT_POST_PREFIX: &str = "admin/edit_post/";
		s = s.trim_start_matches('/').trim_end_matches('/');

		match s {
			"admin" => Ok(Self::Admin),
			"admin/new_post" => Ok(Self::NewPost),
			_ if s.starts_with(EDIT_POST_PREFIX) => {
				let id: u32 = s.trim_start_matches(EDIT_POST_PREFIX).parse().map_err(|_| ())?;
				Ok(Self::EditPost(id))
			},
			_ => Err(())
		}
	}
}

#[cfg(not(target_family = "wasm"))]
impl<H: axum::response::IntoResponse> axum::response::IntoResponse for HtmlOrRedirect<H> {
	fn into_response(self) -> axum::response::Response {
		match self {
			Self::Html(s) => axum::response::Html(s).into_response(),
			Self::Redirect(redir) => axum::response::Redirect::temporary(&redir.url()).into_response()
		}
	}
}
