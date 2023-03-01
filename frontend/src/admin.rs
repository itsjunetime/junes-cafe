use chrono::NaiveDateTime;
use crate::{post_list::{PostViewProvider, PostList}, home::HomeProps};
use shared_data::Post;
use yew::prelude::*;
use std::marker::PhantomData;

#[derive(PartialEq)]
struct AdminPostView;

impl PostViewProvider for AdminPostView {
	fn post_view(post: &Post) -> Html {
		html! {
			<div class="post" id={ format!("post-{}", post.id) }>
				<h2 class="post-title">{ &post.title }</h2>
				{
					post.tags.0.iter().map(|tag|
						html! { <span class="tag">{ tag }</span> }
					).collect::<Html>()
				}
				<div class="post-subtitle-box">
					<span class="post-subtitle">
						{ "by " }
						<strong>{ post.display_user() }</strong>
						{ ", " }
						<strong>{
							NaiveDateTime::from_timestamp_opt(post.created_at as i64, 0)
								.map(|dt| dt.format("%H:%M on %b %-d, %Y").to_string())
								.unwrap_or_else(|| "an unknown time".into())
						}</strong>
					</span>
				</div>
			</div>
		}
	}
}

#[function_component(Admin)]
pub fn admin(props: &HomeProps) -> Html {
	html! {
		<PostList<AdminPostView> page={ props.page } title={ "Admin" } post_view={ PhantomData }/>
	}
}
