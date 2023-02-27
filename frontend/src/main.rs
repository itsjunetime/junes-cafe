use yew_router::prelude::*;
use yew::prelude::*;
use home::Home;
use post::ViewPost;
use edit_post::{EditPostParent, NO_POST};
use auth::AuthView;
use shared_data::Post;

mod post;
mod home;
mod edit_post;
mod style;
mod auth;
mod admin;

#[derive(Clone, Routable, PartialEq)]
enum Route {
	#[not_found]
	#[at("/")]
	Home,
	#[at("/page/:page")]
	HomePage { page: u32 },
	#[at("/post/:id")]
	Post { id: u32 },
	#[at("/edit_post/:id")]
	EditPost { id: u32 },
	#[at("/new_post")]
	NewPost
}

fn switch(route: Route) -> Html {
	match route {
		Route::Home => html! { <Home page={ 0 }/> },
		Route::HomePage { page } => html! { <Home page={ page } /> },
		Route::Post { id } => html! { <ViewPost id={ id } /> },
		Route::EditPost { id } => html! {
			<AuthView>
				<EditPostParent id={ id } />
			</AuthView>
		},
		Route::NewPost => html! {
			<AuthView>
				<EditPostParent id={ NO_POST }/> 
			</AuthView>
		}
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
