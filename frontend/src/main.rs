use yew_router::prelude::*;
use yew::prelude::*;
use edit_post::{EditPostParent, NO_POST};
use shared_data::Post;

mod edit_post;
mod style;
mod post_list;

#[derive(Clone, Routable, PartialEq)]
enum Route {
	#[at("/admin/edit_post/:id")]
	EditPost { id: u32 },
	#[at("/admin/new_post")]
	NewPost,
}

#[expect(clippy::needless_pass_by_value)]
fn switch(route: Route) -> Html {
	match route {
		Route::EditPost { id } => html! {
			<EditPostParent id={ id } />
		},
		Route::NewPost => html! {
			<EditPostParent id={ NO_POST }/>
		},
	}
}

#[derive(Debug)]
pub enum GetPostErr {
	NotFound,
	Unauthorized,
	Other(String)
}

pub fn get_post(id: u32, state: UseStateHandle<Option<Result<Post, GetPostErr>>>) {
	wasm_bindgen_futures::spawn_local(async move {
		let res = match gloo_net::http::Request::get(&format!("/api/post/{id}")).send().await {
			Ok(res) => match res.status() {
				200..300 => res.json::<Post>().await
					.map_err(|e| GetPostErr::Other(format!("There was an error while decoding: {e:?}"))),
				401 => Err(GetPostErr::Unauthorized),
				404 => Err(GetPostErr::NotFound),
				_ => Err(GetPostErr::Other(match res.text().await {
					Err(err) => format!("There was an error getting the response: {err:?}"),
					Ok(text) => format!("There was an error while getting the post: {text}")
				}))
			}
			Err(err) => Err(GetPostErr::Other(format!("{err:?}")))
		};

		state.set(Some(res));
	});
}

pub enum GetPostListErr {
	Unauthorized,
	Other(String)
}

pub fn get_post_list(
	count: usize,
	offset: u32,
	force_logged_in: bool,
	state: UseStateHandle<Option<Result<Vec<Post>, GetPostListErr>>>
) {
	wasm_bindgen_futures::spawn_local(async move {
		let url = format!("/api/posts?count={count}&offset={offset}&force_logged_in={force_logged_in}");
		// This could be such a pretty functional expression but we have to make it an
		// ugly match statement cause we can't have await in blocks
		let res = match gloo_net::http::Request::get(&url).send().await {
			Ok(res) => match res.status() {
				200..300 => res.json::<Vec<Post>>().await
					.map_err(|e| GetPostListErr::Other(format!("There was an error while decoding: {e:?}"))),
				401 => Err(GetPostListErr::Unauthorized),
				status => {
					let text = res.text().await.unwrap_or_else(|e| format!("{e:?}"));
					let error_text = if text.is_empty() {
						"No Error Text (The backend is probably not running)".into()
					} else {
						text
					};

					Err(GetPostListErr::Other(format!("Request returned {status}: {error_text}")))
				}
			},
			Err(err) => Err(GetPostListErr::Other(format!("{err:?}")))
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
