use const_format::concatcp;
use leptos::prelude::*;

pub mod main_page;
pub mod server;
pub mod rsvp_page;
pub mod admin;

#[cfg(not(target_family = "wasm"))]
pub mod app;
#[cfg(not(target_family = "wasm"))]
pub mod faq;

fn view_with_title<T>(title: &'static str, view: T) -> impl IntoView
where
	T: IntoView
{
	view! {
		<!DOCTYPE html>
		<html>
			<head>
				<title>{ title }</title>
			</head>
		</html>
		<body>{ view }</body>
	}
}

pub const SHARED_STYLE: &str = concatcp!(r#"
	@import url('https://fonts.googleapis.com/css2?family=Euphoria+Script&display=swap');
	* {
		font-family: "Euphoria Script", Arial;
	}
	"#,
	SHARED_NO_FONT
);

pub const SHARED_READABLE: &str = concatcp!(
	r#"
	@import url('https://fonts.googleapis.com/css2?family=Amita:wght@400;700&display=swap');
	* {
		font-family: "Amita", seif;
	}
	p, div, span, summary, strong, em {
		font-size: 20px;
	}
	h1 {
		font-size: 44px;
	}
	main {
		margin: 0 auto;
		max-width: 900px
	}
	body {
		margin: 0;
		background-color: var(--beige)
	}
	input, textarea {
		border-radius: 10px;
		border: none;
	}
	input {
		padding: 0 10px;
		font-size: 16px;
	}
	label {
		margin-top: 16px;
	}
	"#,
	SHARED_NO_FONT
);

pub const SHARED_NO_FONT: &str = r#"
* {
	--gold: #8a944d;
	--white: #eff1f3;
	--olive-green: #8d7c3d;
	--soft-brown: #7c59ec;
	--tan: #9b8461;
	--beige: #cbb9b7;
	--dark-brown: #40332a;
	color: var(--dark-brown);
}
"#;
