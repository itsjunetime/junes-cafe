use yew::prelude::*;
use shared_data::Post;
use std::marker::PhantomData;
use crate::{
	get_post_list, 
	style::SharedStyle,
};

// How many posts to request at a time
pub const REQ_BLOCK: usize = 10;

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
	let button_html = if !(show_prev || show_next) {
		html!{{ "That's all!" }}
	} else {
		vec![
			show_prev.then(|| html! { <a href={ format!("/page/{}", props.page - 1) }>{ "< Prev" }</a> }),
			show_next.then(|| html! { <a href={ format!("/page/{}", props.page + 1) }>{ "Next >" }</a> })
		].into_iter()
		.flatten()
		.collect::<Html>()
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
				.post, .home-title, .page-selector {
					max-width: 900px;
					margin: 10px auto;
				}
				.post {
					padding: 8px 10px;
					background-color: var(--main-background);
					border-radius: 8px;
					color: var(--main-text);
				}
				.post-header {
					padding: 0px 6px 4px 6px;
					border-bottom: 1px solid var(--secondary-text);
					color: var(--main-text);
					text-decoration: none;
					display: inline-block;
				}
				.post-title {
					display: inline;
				}
				.post-subtitle-box {
					border-radius: 3px; 
					margin-left: 10px;
					display: inline-block;
				}
				.post-subtitle {
					padding: 4px 0px;
					display: inline-block;
				}
				.post-content {
					padding: 8px 12px;
					position: relative;
					max-height: 240px;
					overflow: hidden;
				}
				.post-content img {
					max-width: 100%;
				}
				.post-content::after {
					content: \"\";
					position: absolute;
					z-index: 1;
					bottom: 0;
					left: 0;
					pointer-events: none;
					background-image: linear-gradient(to bottom, rgba(255, 255, 255, 0), var(--main-background) 90%);
					width: 100%;
					height: 8em;
				}
				.post-footer {
					margin-top: 4px;
				}
				.post-footer span:first-of-type {
					margin-left: 6px;
				}
				.page-selector {
					margin: 12px auto;
					text-align: center;
				}
				"
			}
			</style>
			<div class="home-title">
				<h1>{ &props.title }</h1>
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
