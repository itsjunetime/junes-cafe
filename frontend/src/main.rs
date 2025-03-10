use yew_router::prelude::*;
use yew::prelude::*;
use edit_post::{EditPostParent, NO_POST};
use shared_data::Post;

mod edit_post;
mod style;

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
