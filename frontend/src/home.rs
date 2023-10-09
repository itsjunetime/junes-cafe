use yew::prelude::*;
use crate::post_list::{PostViewProvider, PostList};
use shared_data::Post;
use std::marker::PhantomData;

#[derive(Properties, Clone, PartialEq, Eq)]
pub struct HomeProps {
	pub page: u32
}

#[derive(PartialEq)]
struct HomePostView;

impl PostViewProvider for HomePostView {
	fn post_view(post: &Post) -> Html {
		html! { <>
			<style>
			{
				"
				.post {
					padding: 12px 16px;
					background-color: var(--main-background);
					border-radius: 8px;
					color: var(--main-text);
					margin: 10px auto;
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
					color: var(--title-text);
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
				"
			}
			</style>
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
					<strong>{ crate::title_time_string(post.created_at) }</strong>
					{
						post.tags.0.iter().map(|tag|
							html! { <span class="tag">{ tag }</span> }
						).collect::<Html>()
					}
				</div>
			</div>
		</> }
	}
}

#[function_component(Home)]
pub fn home(props: &HomeProps) -> Html {
	html! {
		<PostList<HomePostView> page={ props.page } title={ "June's Cafe" } post_view={ PhantomData }/>
	}
}
