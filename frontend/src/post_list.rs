use yew::prelude::*;
use shared_data::Post;
use std::marker::PhantomData;
use crate::{
	get_post_list,
	style::SharedStyle, GetPostListErr,
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
	pub force_logged_in: bool,
	pub post_view: PhantomData<P>
}

#[function_component(PostList)]
pub fn post_list<P: PostViewProvider>(props: &PostListProps<P>) -> Html {
	// It would be nice to do [Post; Count] but it could return less than Count posts if there are
	// only that many left, and that should be processed nicely, so oh well.
	let post_list = use_state(|| None);

	// Load the first 10 posts
	{
		let page = props.page;
		let list = post_list.clone();
		let force_logged_in = props.force_logged_in;
		use_effect(move || {
			if list.is_none() {
				get_post_list(REQ_BLOCK, page * REQ_BLOCK as u32, force_logged_in, list);
			}

			|| { }
		});
	}

	// Only have a prev page if this is not the first page
	let show_prev = props.page != 0;
	// Make sure we received the amount of posts we requested
	let show_next = post_list.as_ref().is_some_and(|e| e.as_ref().is_ok_and(|p| p.len() == REQ_BLOCK));

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
		Some(Err(GetPostListErr::Unauthorized)) => {
			// we're assuming that if they get 'unauthorized', then they probably are trying to
			// access the admin page.
			let res = web_sys::window()
				.expect("There's gotta be a window")
				.location()
				.replace("/login?redir_to=/admin");
			return match res {
				Ok(()) => html!{},
				Err(_) => html!{ <>
					<p>{ "We couldn't redirect you to the login page. " }</p>
					<a href="/login?redir_to=/admin">{ "Please click this link." }</a>
				</> }
			}
		},
		Some(Err(GetPostListErr::Other(err))) => html! {
			<><h1>{ "Couldn't get posts" }</h1><p>{ err }</p></>
		},
		Some(Ok(posts)) => posts.iter().map(P::post_view).collect::<Html>(),
	};

	html!{
		<>
			<SharedStyle />
			<style>{ shared_data::POST_LIST_STYLE }</style>
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
