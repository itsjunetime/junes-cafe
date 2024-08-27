use horrorshow::{RenderOnce, TemplateBuffer, html, Raw, helper::doctype, box_html};
use build_info::{VersionControl, GitInfo};

const GITHUB_ICON: &str = include_str!("../../assets/github-mark.svg");
const TWITTER_ICON: &str = include_str!("../../assets/twitter.svg");
const MATRIX_ICON: &str = include_str!("../../assets/matrix.svg");
const RSS_ICON: &str = include_str!("../../assets/rss-icon.svg");

pub struct PostList<'title, C: RenderOnce + 'static> {
	pub content: C,
	pub title: &'title str,
	pub next_page_btn: bool,
	pub current_page: u32
}

impl<C> RenderOnce for PostList<'_, C> where C: RenderOnce + 'static {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		tmpl << html! {
			: doctype::HTML;
			html(lang = "en") {
				head {
					title : self.title;
					style : Raw(shared_data::BASE_STYLE);
					style : Raw(shared_data::POST_LIST_STYLE);
					meta(name = "viewport", content = "width=device-width, initial-scale=1");
				}
				body {
					div(id = "home-title") {
						a(href = "/") {
							h1(id = "title-text") : self.title;
						}
						span(id = "social-icons") {
							a(href = "/index.xml", aria-label = "RSS Feed") : Raw(RSS_ICON);
							a(href = "https://matrix.to/#/@janshai:beeper.com", aria-label = "My Matrix Account") : Raw(MATRIX_ICON);
							a(href = "https://github.com/itsjunetime", aria-label = "My Github") : Raw(GITHUB_ICON);
							a(href = "https://twitter.com/itsjunetime", aria-label = "My Twitter") : Raw(TWITTER_ICON);
						}
					}
					div(id = "posts") : self.content;
					: page_selector_and_credits(self.current_page, self.next_page_btn);
				}
			}
		}
	}
}

fn page_selector_and_credits(current_page: u32, next_page_btn: bool) -> Box<dyn horrorshow::RenderBox> {
	let build = crate::fonts::build();

	box_html! {
		div(class = "page-selector") {
			@ if current_page != 0 {
				a(href = format_args!("/page/{}", current_page - 1)) : "< Prev";
			}
			@ if next_page_btn {
				a(href = format_args!("/page/{}", current_page + 1)) : "Next >";
			}
			@ if current_page == 0 && !next_page_btn {
				: "That's all!";
			}
		}
		div(id = "credits") {
			: format_args!("This was built at {} using rustc {} {}, running ", build.timestamp, build.compiler.channel, build.compiler.version);
			a(href = "https://github.com/itsjunetime/junes-cafe") : "git";
			: format_args!("#{}, using ", match build.version_control {
				Some(VersionControl::Git(GitInfo { ref commit_id, .. })) => commit_id.as_str(),
				_ => "?????"
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
