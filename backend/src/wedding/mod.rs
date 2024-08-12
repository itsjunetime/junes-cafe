pub mod main_page;
pub mod server;
pub mod rsvp_page;
pub mod admin;

#[cfg(not(target_family = "wasm"))]
pub mod app;
#[cfg(not(target_family = "wasm"))]
pub mod faq;

pub const SHARED_STYLE: &str = r#"
@import url('https://fonts.googleapis.com/css2?family=Euphoria+Script&display=swap');
* {
	--gold: #8a944d;
	--white: #eff1f3;
	--olive-green: #8d7c3d;
	--soft-brown: #7c59ec;
	--tan: #9b8461;
	--beige: #cbb9b7;
	--dark-brown: #40332a;
	color: var(--dark-brown);
	font-family: "Euphoria Script", Arial;
}
"#;
