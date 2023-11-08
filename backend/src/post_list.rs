use horrorshow::{RenderOnce, TemplateBuffer, html, Raw, helper::doctype};

const GITHUB_ICON: &str = include_str!("../../assets/github-mark.svg");
const TWITTER_ICON: &str = include_str!("../../assets/twitter.svg");
const MATRIX_ICON: &str = include_str!("../../assets/matrix.svg");

pub struct PostList<C: RenderOnce + 'static> {
	pub content: C,
	pub title: &'static str,
	pub next_page_btn: bool,
	pub current_page: u32
}

impl<C> RenderOnce for PostList<C> where C: RenderOnce + 'static {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		tmpl << html! {
			: doctype::HTML;
			html {
				head {
					style : Raw(shared_data::BASE_STYLE);
					style : Raw(r#"
						#posts {
							margin: 0px auto;
							max-width: max-content;
						}
						#title-text {
							max-width: max-content;
							display: inline-block;
							margin: 10px 0;
						}
						#home-title {
							display: flex;
							justify-content: space-between;
						}
						#home-title > a > svg {
							transform-origin: top center;
						}
						.post, #home-title, .page-selector {
							width: 900px;
						}
						#home-title {
							margin: 20px auto 10px auto;
						}
						.page-selector {
							margin: 10px auto;
						}
						#social-icons {
							align-self: center;
							display: inline-block;
						}
						.page-selector {
							margin: 12px auto;
							text-align: center;
						}
						a {
							text-decoration: none;
						}
					"#);
				}
				body {
					div(id = "home-title") {
						h1(id = "title-text") : self.title;
						span(id = "social-icons") {
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
				}
			}
		}
	}
}
