use chrono::NaiveDateTime;
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
		html! {
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
		}
	}
}

#[function_component(Home)]
pub fn home(props: &HomeProps) -> Html {
	html! {
		<PostList<HomePostView> page={ props.page } title={ "June's Cafe" } post_view={ PhantomData }/>
	}
}
