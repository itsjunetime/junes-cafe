use chrono::NaiveDateTime;
use yew::prelude::*;
use shared_data::Post;
use gloo_net::http::Request;
use crate::style::SharedStyle;

// How many posts to request at a time
const REQ_BLOCK: usize = 10;

#[derive(Properties, Clone, PartialEq, Eq)]
pub struct HomeProps {
	pub page: u32
}

#[function_component(Home)]
pub fn home(props: &HomeProps) -> Html {
	let post_list = use_state(|| Option::<Result<Vec<Post>, String>>::None);

	// Load the first 10 posts
	{
		let page = props.page;
		let list = post_list.clone();
		use_effect(move || {
			if list.is_none() {
				wasm_bindgen_futures::spawn_local(async move {
					let url = format!("/api/posts?count={REQ_BLOCK}&offset={}", page as usize * REQ_BLOCK);
					// This could be such a pretty functional expression but we have to make it an
					// ugly match statement cause we can't have await in blocks
					let res = match Request::get(&url).send().await {
						Ok(res) => if res.ok() {
							res.json::<Vec<Post>>().await
								.map_err(|e| format!("There was an error while decoding: {e:?}"))
						} else {
							let text = res.text().await.unwrap_or_else(|e| format!("{e:?}"));
							Err(format!("Request returned {}: {text}", res.status()))
						},
						Err(err) => Err(format!("{err:?}"))
					};

					list.set(Some(res));
				});
			}

			|| { }
		});
	}

	let posts_html = match &*post_list {
		None => html! { <p>{ "Loading posts..." }</p> },
		Some(Err(err)) => html! { <><h1>{ "Couldn't get posts" }</h1><p>{ err }</p></> },
		Some(Ok(posts)) => html! {
			posts.iter().map(|post| html! {
				<div class="post" id={ format!("post-{}", post.id) }>
					<a href={ format!("/post/{}", post.id) } class="post-header">
						<h2 class="post-title">{ &post.title }</h2>
						<span class="post-subtitle-box">
							<span class="post-subtitle">
								{ "by " }
								<strong>{ post.display_user() }</strong>
								{ ", " }
								<strong>{ post.reading_time.to_string() }</strong>
								{ " minute read " }
							</span>
						</span>
					</a>
					<div class="post-content">
						{ Html::from_html_unchecked(post.html.clone().into()) }
					</div>
					<div class="post-footer">
						{ "Posted at " }
						<strong>{
							NaiveDateTime::from_timestamp_opt(post.created_at as i64, 0)
								.map(|dt| dt.format("%H:%M on %b %-d, %Y").to_string())
								.unwrap_or_else(|| "an unknown time".into())
						}</strong>
						{
							post.tags.0.iter().map(|tag|
								html! { <span class="tag">{ tag }</span> }
							).collect::<Html>()
						}
					</div>
				</div>
			})
			.collect::<Html>()
		}
	};

	// Only have a prev page if this is not the first page
	let show_prev = props.page != 0;
	// Make sure we received the amount of posts we requested
	let show_next = post_list.as_ref().map_or(false, |e| e.as_ref().map_or(false, |p| p.len() == REQ_BLOCK));

	// Construct the buttons on the bottom of the page based on if we should show them
	let mut buttons = Vec::with_capacity(2);

	if show_prev || show_next {
		if show_prev {
			buttons.push(html! {
				<a href={ format!("/page/{}", props.page - 1) }>{ "< Prev" }</a>
			})
		}
		if show_next {
			buttons.push(html! {
				<a href={ format!("/page/{}", props.page + 1) }>{ "Next >" }</a>
			})
		}
	} else {
		buttons.push(html!{{ "That's all!" }})
	}

	let button_html = buttons.into_iter().collect::<Html>();

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
				<h1>{ "June's Cafe" }</h1>
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
