use yew::prelude::*;
use gloo_console::log;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(PartialEq, Eq, Debug)]
enum LoginStatus {
	AwaitingResponse,
	FailedAutoLogin,
	InternalError,
	LoggedIn,
	BadAuth,
	NoPasswordProvided
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

	let login_status = use_state(|| LoginStatus::AwaitingResponse);
	// (username, password)
	let creds = use_state(|| (String::new(), String::new()));

	let login_clone = login_status.clone();

	let user_creds = creds.clone();
	let username_input = Callback::from(move |e: Event|
		if let Some(input) = e.target()
			.and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
				user_creds.set((input.value(), user_creds.1.clone()));
			}
	);

	let pass_creds = creds.clone();
	let password_input = Callback::from(move |e: Event|
		if let Some(input) = e.target()
			.and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
				pass_creds.set((pass_creds.0.clone(), input.value()));
			}
	);

	let submit_click = Callback::from(move |_| {
		let login_reclone = login_clone.clone();

		let auth_header = format!("Basic {}", base64::encode(format!("{}:{}", creds.0, creds.1)));

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
		if *auto_login == LoginStatus::AwaitingResponse {
			wasm_bindgen_futures::spawn_local(async move {
				let request = gloo_net::http::Request::get("/api/login")
					.header("Authorization", "Basic Og==");
				match request.send().await {
					Ok(r) if r.status() == 200 => auto_login.set(LoginStatus::LoggedIn),
					res => {
						log!(format!("Auto-login failed: {res:?}"));
						auto_login.set(LoginStatus::FailedAutoLogin)
					}
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
		LoginStatus::AwaitingResponse | LoginStatus::FailedAutoLogin => html! {
			<>
				<h1>{ "Login" }</h1>
				<input placeholder="username" onchange={ username_input } />
				<input placeholder="password" type="password" onchange={ password_input } />
				<button onclick={ submit_click }>{ "Login" }</button>
			</>
		}
	}
}
