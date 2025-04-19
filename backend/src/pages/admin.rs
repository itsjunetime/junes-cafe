use axum_sqlx_tx::Tx;
use backend::auth::get_username;
use horrorshow::{html, Raw, RenderOnce, Template};
use shared_data::Post;
use sqlx::Postgres;
use tower_sessions::Session;

use crate::{blog_api::get_post_list_with_user, post_list::PostList, robots::desc_if_debug};

use super::{HtmlOrRedirect, RedirLocation};

pub async fn admin(session: Session, mut tx: Tx<Postgres>) -> HtmlOrRedirect<Box<str>> {
	let Some(username) = get_username(&session).await else {
		return HtmlOrRedirect::Redirect(super::Redirect {
			force_login: true,
			redir_to: RedirLocation::Admin
		});
	};

	match get_post_list_with_user(&mut tx, u32::MAX, 0, Some(username)).await {
		Ok(p) => HtmlOrRedirect::Html(
			PostList {
				content: AdminPosts(p),
				title: "Admin",
				next_page_btn: false,
				current_page: 0
			}.into_string()
			.unwrap()
			.into()
		),
		Err(e) => HtmlOrRedirect::Html(
			PostList {
				content: desc_if_debug(e).to_string(),
				title: "Admin",
				next_page_btn: false,
				current_page: 0
			}.into_string()
			.unwrap()
			.into()
		)
	}
}

struct AdminPosts(Vec<Post>);

impl RenderOnce for AdminPosts {
	fn render_once(self, tmpl: &mut horrorshow::TemplateBuffer<'_>)
	where
		Self: Sized
	{
		tmpl << html! {
			style: Raw("
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
			");
			@ for post in self.0 {
				div(class = "post", id = format_args!("post-{}", post.id)) {
					div(class = "post-top") {
						a(href = format_args!("/post/{}", post.id)) {
							h2(class = "post-title"): post.title
						}

						div(class = "post-subtitle-box") {
							span(class = "post-subtitle") {
								: "by";
								strong: Post::display_user(&post.username);
								: ", ";
								strong: shared_data::title_time_string(post.created_at);
							}
						}
					}
					hr;

					div(class = "post-bottom") {
						span(class = "tags") {
							@ for tag in post.tags.0 {
								span(class = "tag"): tag;
							}
						}

						span(class = "action-links") {
							a(class = "action-link", href = format_args!("/post/{}", post.id)): "View";
							a(class = "action-link", href = format_args!("/admin/edit_post/{}", post.id)): "Edit"
						}
					}
				}
			}
		}
	}
}
