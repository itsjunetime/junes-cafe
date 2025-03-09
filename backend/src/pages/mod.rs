use std::{borrow::Cow, str::FromStr};

use axum::response::{IntoResponse, Redirect, Html};

pub mod login;
pub mod admin;

pub enum HtmlOrRedirect {
    Html(Box<str>),
    Redirect {
        force_login: bool,
        redir_to: RedirLocation
    }
}

#[derive(Default)]
pub enum RedirLocation {
    #[default]
    Admin,
    EditPost(u32),
    NewPost
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

impl IntoResponse for HtmlOrRedirect {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Html(s) => Html(s).into_response(),
            Self::Redirect { force_login, redir_to } => {
                let mut eventual_dest: Cow<str> = match redir_to {
                    RedirLocation::Admin => "/admin".into(),
                    RedirLocation::NewPost => "/admin/new_post".into(),
                    RedirLocation::EditPost(id) => format!("/admin/edit_post/{id}").into(),
                };

                if force_login {
                    eventual_dest = format!("/login?redir_to={eventual_dest}").into();
                }

                Redirect::temporary(&eventual_dest).into_response()
            }
        }
    }
}
