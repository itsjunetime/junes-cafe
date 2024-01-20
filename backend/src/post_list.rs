use horrorshow::{RenderOnce, TemplateBuffer, html, Raw, helper::doctype};
use build_info::{VersionControl, GitInfo};

const GITHUB_ICON: &str = include_str!("../../assets/github-mark.svg");
const TWITTER_ICON: &str = include_str!("../../assets/twitter.svg");
const MATRIX_ICON: &str = include_str!("../../assets/matrix.svg");
const RSS_ICON: &str = include_str!("../../assets/rss-icon.svg");

pub struct PostList<C: RenderOnce + 'static> {
	pub content: C,
	pub title: &'static str,
	pub next_page_btn: bool,
	pub current_page: u32
}

impl<C> RenderOnce for PostList<C> where C: RenderOnce + 'static {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		let build_info = crate::fonts::build();
		let compiler_info = &build_info.compiler;
		let unknown_commit = "?????".to_string();

		tmpl << html! {
			: doctype::HTML;
			html {
				head {
					title : "itsjuneti.me";
					style : Raw(shared_data::BASE_STYLE);
					style : Raw(shared_data::POST_LIST_STYLE);
				}
				body {
					div(id = "home-title") {
						a(href = "/") {
							h1(id = "title-text") : self.title;
						}
						span(id = "social-icons") {
							a(href = "/index.xml") : Raw(RSS_ICON) ;
							a(href = "https://matrix.to/#/@janshai:beeper.com") : Raw(MATRIX_ICON);
							a(href = "https://github.com/itsjunetime") : Raw(GITHUB_ICON);
							a(href = "https://twitter.com/itsjunetime") : Raw(TWITTER_ICON);
						}
					}
					div(id = "posts") : self.content;
					div(class = "page-selector") {
						@ if self.current_page != 0 {
							a(href = format_args!("/page/{}", self.current_page - 1)) : "< Prev";
						}
						@ if self.next_page_btn {
							a(href = format_args!("/page/{}", self.current_page + 1)) : "Next >";
						}
						@ if self.current_page == 0 && !self.next_page_btn {
							: "That's all!";
						}
					}
					div(id = "credits") {
						: format!("This was built at {} using rustc {} {}, running ", build_info.timestamp, compiler_info.channel, compiler_info.version);
						a(href = "https://github.com/itsjunetime/junes-cafe") : "git";
						: format!("#{}, using ", match build_info.version_control {
							Some(VersionControl::Git(GitInfo { ref commit_id, .. })) => commit_id,
							_ => &unknown_commit
						});
						a(href = "https://github.com/tokio-rs/axum") : "axum";
						: ", ";
						a(href = "https://tokio.rs") : "tokio";
						: ", ";
						a(href = "https://github.com/Stebalien/horrorshow-rs") : "horrorshow";
						: ", and ";
						a(href = "https://yew.rs") : "yew";
						br;
						: "You can find more info on the ";
						a(href = "/licenses") : "license page";
						: " :)";
					}
				}
			}
		}
	}
}
