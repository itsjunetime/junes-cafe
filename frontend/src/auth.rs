use yew::prelude::*;
use crate::style::SharedStyle;
use gloo_console::log;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(PartialEq, Eq, Debug)]
enum LoginStatus {
	NoAction,
	AutoLoginAttempted,
	InternalError,
	LoggedIn,
	BadAuth,
	NoPasswordProvided
}

#[derive(Clone, Default)]
struct Credentials {
	username: String,
	password: String
}

#[derive(Properties, PartialEq)]
pub struct AuthProps {
	pub children: Children
}

#[function_component(AuthView)]
pub fn auth_view(props: &AuthProps) -> Html {
	// So this whole thing is not really super secure 'cause it's all running in wasm client-side
	// and technically someone with know-how could mess with that but the important part is that
	// the true authentication is happening server-side with every API request, so if they manage
	// to view this page, that's just a fun little bonus for them and they don't actually gain any
	// access

	let login_status = use_state(|| LoginStatus::NoAction);
	let creds = use_state(Credentials::default);

	let login_clone = login_status.clone();

	macro_rules! input_callback{ ($item:ident) => {{
		let priv_creds = creds.clone();
		Callback::from(move |e: Event|
			if let Some($item) = e.target()
				.and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
				.map(|i| i.value()) {
					priv_creds.set(Credentials { $item, ..(*priv_creds).clone() });
				}
		)
	}}}

	let username_input = input_callback!(username);
	let password_input = input_callback!(password);

	let submit_click = Callback::from(move |_| {
		let login_reclone = login_clone.clone();

		let auth_header = format!("Basic {}", base64::encode(format!("{}:{}", creds.username, creds.password)));

		wasm_bindgen_futures::spawn_local(async move {
			let res = gloo_net::http::Request::get("/api/login")
				.header("Authorization", &auth_header)
				.send()
				.await
				.map_or_else(
					|e| {
						log!(format!("Error sending request: {e:?}"));
						LoginStatus::InternalError
					},
					|r| match r.status() {
						200 => LoginStatus::LoggedIn,
						401 => LoginStatus::BadAuth,
						412 => LoginStatus::NoPasswordProvided,
						_ => LoginStatus::InternalError
					}
				);

			login_reclone.set(res);
		})
	});

	// And now, just to see if we've already logged in, we hit /api/login before even typing
	// anything to see if we can automatically redirect
	let auto_login = login_status.clone();
	use_effect(move || {
		if *auto_login == LoginStatus::NoAction {
			auto_login.set(LoginStatus::AutoLoginAttempted);

			wasm_bindgen_futures::spawn_local(async move {
				let request = gloo_net::http::Request::get("/api/login")
					.header("Authorization", "Basic Og==");

				match request.send().await {
					Ok(r) if r.status() == 200 => auto_login.set(LoginStatus::LoggedIn),
					res => log!(format!("Auto-login failed: {res:?}")),
				}
			});
		}
	});

	match *login_status {
		LoginStatus::InternalError => html! { "Something went wrong, try again later :/" },
		LoginStatus::BadAuth => html! { "Incorrect username or password" },
		LoginStatus::NoPasswordProvided => html! { "No password was provided" },
		LoginStatus::LoggedIn => html! {
			{ for props.children.iter() }
		},
		LoginStatus::NoAction | LoginStatus::AutoLoginAttempted => html! {
			<>
				<SharedStyle/>
				<style>{
					"
					button:hover {
						background-color: #00000000;
					}
					#login-form {
						max-width: max-content;
						margin: auto;
					}
					input {
						font-size: 20px;
					}
					"
				}</style>
				<div id="login-form">
					<h1>{ "Login" }</h1>
					<br/>
					<input placeholder="username" onchange={ username_input } />
					<br/><br/>
					<input placeholder="password" type="password" onchange={ password_input } />
					<br/><br/><br/>
					<button onclick={ submit_click }>{ "Login" }</button>
				</div>
			</>
		}
	}
}
