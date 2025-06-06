use tower_sessions::Session;
use axum_sqlx_tx::Tx;
use axum::{response::Html, extract::Path, http::StatusCode};
use sqlx::Postgres;
use shared_data::Post;
use horrorshow::{RenderOnce, TemplateBuffer, html, Raw, Template, helper::doctype};
use backend::auth::get_username;

use crate::blog_api::get_post;

pub async fn get_post_view(
	session: Session,
	tx: Tx<Postgres>,
	Path(id): Path<u32>
) -> Result<Html<String>, StatusCode> {
	let username = get_username(&session).await;
	let Ok(post) = get_post(session, tx, Path(id)).await else {
		return Err(StatusCode::NOT_FOUND);
	};

	let can_edit = username.is_some_and(|username| username == post.username);
	let view = PostView { post, can_edit };
	Ok(Html(view.into_string().unwrap()))
}

struct PostView {
	post: Post,
	can_edit: bool
}

impl RenderOnce for PostView {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		tmpl << html! {
			: doctype::HTML;
			html(lang = "en") {
				head {
					title : &self.post.title;
					style : Raw(shared_data::BASE_STYLE);
					style : Raw(r"
						#post-content {
							max-width: 790px;
							margin: 10px auto;
						}
						#post-header * {
							color: var(--secondary-text);
						}
						#back-button {
							height: 0;
							display: block;
							right: 30px;
							position: relative;
							top: 10px;
							text-decoration: none;
						}
						#post-title {
							color: var(--title-text);
						}
						#title-row {
							display: flex;
							justify-content: space-between;
						}
						#title-row * {
							margin: 0px;
							padding: 8px 0px;
						}
						#post-text {
							padding: 12px;
						}
					&	#post-text img {
							max-width: 100%;
						}
						#tag-title {
							color: var(--secondary-text);
						}
						.tag-group {
							margin-top: 6px;
						}
					");
					meta(name = "viewport", content = "width=device-width, initial-scale=1");
				}
				body {
					div(id = "post-content") {
						span(id = "post-header") {
							a(href = "/", id = "back-button") : "←";
							span(id = "title-row") {
								h2(id = "post-title") : self.post.title;
								@ if self.can_edit {
									a(href = format_args!("/admin/edit_post/{}", self.post.id)) : "edit";
								}
							}
							span {
								: "At ";
								strong : shared_data::title_time_string(self.post.created_at);
								: " by ";
								strong : Post::display_user(&self.post.username);
								: "; ";
								@ if self.post.reading_time == 0 {
									: "a quick read";
								} else {
									: self.post.reading_time;
									: " minute read";
								}
							}
						}
						div(id = "post-text") : Raw(self.post.html);
						@ if !self.post.tags.0.is_empty() {
							br; br;
							div(id = "tags") {
								span(id = "tag-title") : "tags";
								span(class = "tag-group") {
									@ for tag in self.post.tags.0 {
										span(class = "tag") : tag;
									}
								}
							}
						}
					}
				}
			}
		}
	}
}
