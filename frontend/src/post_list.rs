use yew::prelude::*;
use shared_data::Post;
use std::marker::PhantomData;
use crate::{
	get_post_list, 
	style::SharedStyle,
};

// How many posts to request at a time
pub const REQ_BLOCK: usize = 10;

const GITHUB_ICON: &str = include_str!("../../assets/github-mark.svg");
const TWITTER_ICON: &str = include_str!("../../assets/twitter.svg");
const MATRIX_ICON: &str = include_str!("../../assets/matrix.svg");

pub trait PostViewProvider: PartialEq {
	fn post_view(post: &Post) -> Html;
}

#[derive(Properties, PartialEq)]
pub struct PostListProps<P: PostViewProvider> {
	pub page: u32,
	pub title: String,
	pub post_view: PhantomData<P>
}

#[function_component(PostList)]
pub fn post_list<P: PostViewProvider>(props: &PostListProps<P>) -> Html {
	// It would be nice to do [Post; Count] but it could return less than Count posts if there are
	// only that many left, and that should be processed nicely, so oh well.
	let post_list = use_state(|| Option::<Result<Vec<Post>, String>>::None);

	// Load the first 10 posts
	{
		let page = props.page;
		let list = post_list.clone();
		use_effect(move || {
			if list.is_none() {
				get_post_list(REQ_BLOCK, page * REQ_BLOCK as u32, list);
			}

			|| { }
		});
	}

	// Only have a prev page if this is not the first page
	let show_prev = props.page != 0;
	// Make sure we received the amount of posts we requested
	let show_next = post_list.as_ref().map_or(false, |e| e.as_ref().map_or(false, |p| p.len() == REQ_BLOCK));

	// Construct the buttons on the bottom of the page based on if we should show them
	let button_html = if show_prev || show_next {
		vec![
			show_prev.then(|| html! { <a href={ format!("/page/{}", props.page - 1) }>{ "< Prev" }</a> }),
			show_next.then(|| html! { <a href={ format!("/page/{}", props.page + 1) }>{ "Next >" }</a> })
		].into_iter()
		.flatten()
		.collect::<Html>()
	} else {
		html!{{ "That's all!" }}
	};

	let posts_html = match &*post_list {
		None => html! { <p>{ "Loading posts..." }</p> },
		Some(Err(err)) => html! { <><h1>{ "Couldn't get posts" }</h1><p>{ err }</p></> },
		Some(Ok(posts)) => posts.iter().map(P::post_view).collect::<Html>(),
	};

	html!{
		<>
			<SharedStyle />
			<style>
			{
				"
				body {
					font-family: Arial
				}
				#posts {
					margin: 0px auto;
					max-width: max-content;
				}
				#title-text {
					max-width: max-content;
					display: inline-block;
					margin: 10px 0;
				}
				#home-title {
					display: flex;
					justify-content: space-between;
				}
				#home-title > a > svg {
					transform-origin: top center;
				}
				.post, #home-title, .page-selector {
					max-width: 900px;
				}
				#home-title {
					margin: 20px auto 10px auto;
				}
				.page-selector {
					margin: 10px auto;
				}
				#social-icons {
					align-self: center;
					display: inline-block;
				}
				.page-selector {
					margin: 12px auto;
					text-align: center;
				}
				a {
					text-decoration: none;
				}
				"
			}
			</style>
			<div id="home-title">
				<h1 id="title-text">{ &props.title }</h1>
				<span id="social-icons">
					<a href="https://matrix.to/#/@janshai:beeper.com">{ Html::from_html_unchecked(MATRIX_ICON.into()) }</a>
					<a href="https://github.com/itsjunetime">{ Html::from_html_unchecked(GITHUB_ICON.into()) }</a>
					<a href="https://twitter.com/itsjunetime">{ Html::from_html_unchecked(TWITTER_ICON.into()) }</a>
				</span>
			</div>
			<div id="posts">
				{ posts_html }
			</div>
			<div class="page-selector">
				{ button_html }
			</div>
		</>
	}
}
