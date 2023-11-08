use yew_router::prelude::*;
use yew::prelude::*;
use edit_post::{EditPostParent, NO_POST};
use auth::AuthView;
use shared_data::Post;
use admin::Admin;

mod edit_post;
mod style;
mod auth;
mod admin;
mod post_list;

#[derive(Clone, Routable, PartialEq)]
enum Route {
	#[at("/admin/edit_post/:id")]
	EditPost { id: u32 },
	#[at("/admin/new_post")]
	NewPost,
	#[at("/admin/:page")]
	Admin { page: u32 },
	#[at("/admin")]
	AdminHome
}

#[allow(clippy::needless_pass_by_value)]
fn switch(route: Route) -> Html {
	match route {
		Route::EditPost { id } => html! {
			<AuthView>
				<EditPostParent id={ id } />
			</AuthView>
		},
		Route::NewPost => html! {
			<AuthView>
				<EditPostParent id={ NO_POST }/>
			</AuthView>
		},
		Route::Admin { page } => html! {
			<AuthView>
				<Admin page={ page }/>
			</AuthView>
		},
		Route::AdminHome => switch(Route::Admin { page: 0 })
	}
}

#[derive(Debug)]
pub enum GetPostErr {
	NotFound,
	Other(String)
}

pub fn get_post(id: u32, state: UseStateHandle<Option<Result<Post, GetPostErr>>>) {
	wasm_bindgen_futures::spawn_local(async move {
		let res = match gloo_net::http::Request::get(&format!("/api/post/{id}")).send().await {
			Ok(res) => if res.ok() {
				res.json::<Post>().await
					.map_err(|e| GetPostErr::Other(format!("There was an error while decoding: {e:?}")))
			} else if res.status() == 404 {
				Err(GetPostErr::NotFound)
			} else {
				Err(GetPostErr::Other(match res.text().await {
					Err(err) => format!("There was an error getting the response: {err:?}"),
					Ok(text) => format!("There was an error while getting the post: {text}")
				}))
			}
			Err(err) => Err(GetPostErr::Other(format!("{err:?}")))
		};

		state.set(Some(res));
	});
}

pub fn get_post_list(count: usize, offset: u32, state: UseStateHandle<Option<Result<Vec<Post>, String>>>) {
	wasm_bindgen_futures::spawn_local(async move {
		let url = format!("/api/posts?count={count}&offset={offset}");
		// This could be such a pretty functional expression but we have to make it an
		// ugly match statement cause we can't have await in blocks
		let res = match gloo_net::http::Request::get(&url).send().await {
			Ok(res) => if res.ok() {
				res.json::<Vec<Post>>().await
					.map_err(|e| format!("There was an error while decoding: {e:?}"))
			} else {
				let text = res.text().await.unwrap_or_else(|e| format!("{e:?}"));
				let error_text = if text.is_empty() {
					"No Error Text (The backend is probably not running)".into()
				} else {
					text
				};

				Err(format!("Request returned {}: {error_text}", res.status()))
			},
			Err(err) => Err(format!("{err:?}"))
		};

		state.set(Some(res));
	});
}

#[function_component(Frontend)]
pub fn frontend() -> Html {
	html! {
		<BrowserRouter>
			<Switch<Route> render={switch} />
		</BrowserRouter>
	}
}

fn main() {
	yew::Renderer::<Frontend>::new().render();
}
