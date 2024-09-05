use crate::post_list::{PostViewProvider, PostList};
use shared_data::Post;
use yew::prelude::*;
use std::marker::PhantomData;

#[derive(PartialEq)]
struct AdminPostView;

impl PostViewProvider for AdminPostView {
	fn post_view(post: &Post) -> Html {
		html! {
			<div class="post" id={ format!("post-{}", post.id) }>
				<div class="post-top">
					<a href={ format!("/post/{}", post.id) }>
						<h2 class="post-title">{ &post.title }</h2>
					</a>
					<div class="post-subtitle-box">
						<span class="post-subtitle">
							{ "by " }
							<strong>{ post.display_user() }</strong>
							{ ", " }
							<strong>{ shared_data::title_time_string(post.created_at) }</strong>
						</span>
					</div>
				</div>
				<hr/>
				<div class="post-bottom">
					<span class="tags">
						{
							post.tags.0.iter().map(|tag|
								html! { <span class="tag">{ tag }</span> }
							).collect::<Html>()
						}
					</span>
					<span class="action-links">
						<a class="action-link" href={ format!("/post/{}", post.id) }>{ "View" }</a>
						<a class="action-link" href={ format!("/admin/edit_post/{}", post.id) }>{ "Edit" }</a>
					</span>
				</div>
			</div>
		}
	}
}

#[derive(Properties, PartialEq)]
pub struct AdminProps {
	pub page: u32
}

#[function_component(Admin)]
pub fn admin(props: &AdminProps) -> Html {
	html! { <>
		<style>
		{ "
		.post {
			display: flex;
			flex-flow: wrap;
			margin: 16px auto;
			border: 2px solid var(--secondary-background);
			border-radius: 14px;
			padding: 16px 20px;
		}
		.post hr {
			flex-basis: 100%;
			margin: 0;
			border: 0;
		}
		.post-title {
			margin: 0px;
			align-items: flex-end;
		}
		.post-bottom, .post-top {
			display: flex;
			justify-content: space-between;
			flex-grow: 100;
		}
		.post-bottom {
			margin: 8px 0px 4px 0px;
		}
		.tags, .action-links {
			display: flex;
			gap: 8px;
		}
		.tags {
			justify-content: flex-start;
		}
		.tag {
			margin-left: 0px !important;
		}
		.action-links {
			justify-content: flex-end;
		}
		a {
			text-decoration: none;
		}
		a.action-link {
			padding: 4px 8px;
			border-radius: 6px;
			border: 2px solid var(--secondary-background);
		}
		a.action-link:hover {
			background-color: var(--secondary-background);
		}
		" }
		</style>
		<PostList<AdminPostView> page={ props.page } title={ "Admin" } post_view={ PhantomData }/>
	</>}
}
