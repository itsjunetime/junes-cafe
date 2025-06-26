use axum_extra::extract::Host;
use tower_sessions::Session;
use sqlx::Postgres;
use axum_sqlx_tx::Tx;
use axum::{response::Html, extract::Path};
use horrorshow::{html, Raw, RenderOnce, TemplateBuffer, Template};
use crate::{blog_api::get_post_list, post_list::PostList, Post};

pub async fn get_home_view(session: Session, tx: Tx<Postgres>, host: Host) -> Html<String> {
	get_page_view(session, tx, Path(0), host).await
}

pub async fn get_page_view(
	session: Session,
	mut tx: Tx<Postgres>,
	Path(page): Path<u32>,
	Host(host): Host,
) -> Html<String> {
	let posts = get_post_list(Some(session), &mut tx, 10, page * 10).await;
	let show_next = posts.as_ref().is_ok_and(|p| p.len() == 10);

	println!("host: {host}");

	Html(PostList {
		content: Posts(posts),
		title: &host,
		next_page_btn: show_next,
		current_page: page
	}.into_string()
	.unwrap())
}

struct Posts(Result<Vec<Post>, sqlx::Error>);

impl RenderOnce for Posts {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		match self.0 {
			Err(sqlx::Error::RowNotFound) => tmpl << html! { },
			Err(_) => tmpl << html! { span { : "Ran into an error while retrieving posts" } },
			Ok(posts) => tmpl << html! {
				style : Raw(r#"
				.post {
					padding: 12px;
					background-color: var(--main-background);
					border-radius: 8px;
					margin: 10px auto;
				}
				.post-header {
					padding: 0px 6px 4px 6px;
					border-bottom: 1px solid var(--secondary-text);
					display: inline-block;
				}
				.post-title {
					display: inline;
					color: var(--title-text);
				}
				.post-subtitle-box {
					margin-left: 10px;
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
					content: "";
					position: absolute;
					z-index: 10;
					bottom: 0;
					left: 0;
					pointer-events: none;
					background-image: linear-gradient(to bottom, rgba(255, 255, 255, 0), var(--main-background) 90%);
					width: 100%;
					height: 8em;
				}
				.post-footer {
					margin-top: 10px;
					display: flex;
					justify-content: space-between;
					flex-wrap: wrap;
					row-gap: 10px;
				}
				.post-time {
					align-self: flex-end;
				}
				"#);
				@ for post in posts {
					div(class = "post", id = format_args!("post-{}", post.id)) {
						a(href = format_args!("/post/{}", post.id), class="post-header") {
							h2(class = "post-title") : post.title.clone();
							span(class = "post-subtitle-box") {
								span(class = "post-subtitle") {
									: " by ";
									: Post::display_user(&post.username);
									: ", ";
									@ if post.reading_time == 0 {
										: "a quick read";
									} else {
										strong {
											: post.reading_time
										}
										: " minute read";
									}
								}
							}
						}
						div(class = "post-content") : Raw(
							// only do the first 5 paragraphs since it's gonna hide past that
							post.html.split("</p>")
								.take(4)
								.collect::<Vec<&str>>()
								.join("</p>")
								 + "</p>"
						);
						div(class = "post-footer") {
							span(class = "post-time") {
								: "Posted at ";
								strong : shared_data::title_time_string(post.created_at);
							}
							span(class = "tag-group") {
								@ for tag in post.tags.0 {
									span(class = "tag") {
										: tag;
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
