use axum::{response::Html, extract::Path};
use horrorshow::{html, Raw, helper::doctype, RenderOnce, TemplateBuffer, Template};
use std::sync::OnceLock;
use crate::post_list::PostList;

static MAPLE_MONO_LIGHT: &[u8] = include_bytes!("../../fonts/maple-mono/woff2/MapleMono-Light.woff2");
static ISENHEIM_REGULAR: &[u8] = include_bytes!("../../fonts/isenheim/fonts/OpenType-PS/Isenheim_Regulier.otf");

pub async fn get_font(Path(id): Path<String>) -> &'static [u8] {
	// error tolerance babey
	if id.contains("maple") {
		MAPLE_MONO_LIGHT
	} else {
		ISENHEIM_REGULAR
	}
}

build_info::build_info!(pub fn build);

pub async fn get_license_page() -> Html<String> {
	Html(PostList {
		content: LicensePage,
		title: "itsjuneti.me (licenses)",
		next_page_btn: false,
		current_page: 0
	}.into_string()
	.unwrap())
}

struct LicensePage;

impl RenderOnce for LicensePage {
	fn render_once(self, tmpl: &mut TemplateBuffer) {
		// We can make this a OnceLock because it'll only change if dependencies change, and those
		// aren't gonna change unless we rebuild it
		static LICENSE_HTML: OnceLock<String> = OnceLock::new();

		tmpl << html! {
			: Raw(LICENSE_HTML.get_or_init(|| {
				let crate_info = &build().crate_info;
				let no_license = "No license".to_string();

				html! {
					: doctype::HTML;
					html {
						head {
							title : "itsjuneti.me (licenses)";
							style : Raw(shared_data::BASE_STYLE);
						}
						body {
							h1 : "Dependencies";
							ul {
								// don't want to show shared_data as a real dependency
								@ for krate in crate_info.dependencies.iter().filter(|dep| dep.name != "shared_data") {
									li {
										a(href = format_args!("https://crates.io/crates/{}", krate.name)) : &krate.name;
										: ": ";
										: krate.license.as_ref().unwrap_or(&no_license);
									}
								}
							}
							br;

							h1: "Fonts";
							ul {
								li {
									a(href = "https://www.tunera.xyz/fonts/isenheim/") : "Isenheim";
									: ": ";
									a(href = "https://www.tunera.xyz/licenses/sil-open-font-license-1.1/") : "SIL Open Font License v1.1";
								}
								li {
									a(href = "https://github.com/subframe7536/maple-font") : "Maple Mono";
									: ": ";
									a(href = "https://github.com/subframe7536/maple-font/blob/main/OFL.txt") : "SIL Open Font License v1.1";
								}
							}
						}
					}
				}.to_string()
			}))
		};
	}
}
