use axum_sessions::extractors::ReadableSession;
use axum_sqlx_tx::Tx;
use axum::{response::Html, extract::Path, http::StatusCode};
use shared_data::sqlx::Postgres;
use shared_data::Post;
use horrorshow::{RenderOnce, TemplateBuffer, html, Raw, Template, helper::doctype};

pub async fn get_post_view(
	session: ReadableSession,
	tx: Tx<Postgres>,
	Path(id): Path<i32>
) -> Result<Html<String>, StatusCode> {
	let Ok(post) = crate::get_post(session, tx, Path(id)).await else {
		return Err(StatusCode::NOT_FOUND);
	};

	Ok(Html(PostView(post).into_string().unwrap()))
}

struct PostView(Post);

impl RenderOnce for PostView {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		let post = self.0;
		let user = post.display_user().to_owned();
		tmpl << html! {
			: doctype::HTML;
			html {
				head {
					style : Raw(shared_data::BASE_STYLE);
					style : Raw(r#"
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
							top: 24px;
							text-decoration: none;
						}
						#post-title {
							color: var(--title-text);
						}
						#post-text {
							padding: 12px 12px;
						}
						#post-text img {
							max-width: 100%;
						}
						#tag-title {
							color: var(--secondary-text);
						}
						#tag-title ~ br {
							margin-bottom: 10px;
						}
					"#);
				}
				body {
					div(id = "post-content") {
						span(id = "post-header") {
							a(href = "/", id = "back-button") : "←";
							h2(id = "post-title") : post.title;
							span {
								: "At ";
								strong : shared_data::title_time_string(post.created_at);
								: " by ";
								strong : user;
								: "; ";
								@ if post.reading_time == 0 {
									: "a quick read";
								} else {
									: post.reading_time;
									: " minute read";
								}
							}
						}
						br; br;
						div(id = "post-text") : Raw(post.html);
						@ if !post.tags.0.is_empty() {
							br; br;
							div(id = "tags") {
								span(id = "tag-title") : "Tags";
								br;
								@ for tag in post.tags.0 {
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
