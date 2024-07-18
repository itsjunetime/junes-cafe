// use leptos::prelude::*;
use leptos::*;

#[cfg(not(target_family = "wasm"))]
use const_format::concatcp;
#[cfg(not(target_family = "wasm"))]
use super::server::AddAnnouncementReq;
#[cfg(not(target_family = "wasm"))]
use leptos_router::ActionForm;

#[cfg(not(target_family = "wasm"))]
const STYLE: &str = concatcp!(super::SHARED_STYLE, r#"
body {
	background-color: var(--white);
	margin: 0 0 10px 0;
}
input {
	border-radius: 16px;
	background-color: var(--white);
	border: 1px solid gray;
	margin: -2px 0 10px 0;
	font-size: 20px;
	padding: 4px 12px 3px 12px;
}
#form-inputs, form > input, #form-side > div > *, #announcement-form > div {
	margin-left: 50%;
	transform: translateX(-50%);
	display: inline-block;
}
#form-inputs {
	margin-bottom: 6px;
	width: max-content;
	max-width: 60%;
}
#form-inputs > input {
	width: 100%;
}
#img-and-overlay > * {
	width: 100%;
	object-fit: cover;
}
#img-and-overlay {
	position: relative;
}
#top-left-text {
	top: 40px;
	left: 80px;
	font-size: 80px;
}
#bottom-right-text {
	bottom: 10px;
	right: 10px;
	color: var(--white);
	text-align: end;
	text-shadow: 0px 0px 14px rgba(0, 0, 0, 0.9);
	font-size: 64px;
}
#img-and-overlay > div {
	z-index: 10;
	position: absolute;
	width: max-content;
}
#second-row {
	display: grid;
	grid-template-columns: 2fr 3fr;
	margin: 12px;
	grid-row-gap: 10px;
}
#second-row > img {
	border-radius: 12px;
}
#form-side {
	display: grid;
	grid-template-rows: auto auto auto;
	font-size: 24px;
	margin: 20px 0;
}
#form-side > div {
	margin: auto 0;
}
#form-side > img {
	position: absolute;
	width: 12%;
}
#form-section > h1 {
	text-align: center;
}
#top-left-corner-deco {
	margin-left: 36px;
}
#bottom-right-corner-deco {
	rotate: 180deg;
	right: 36px;
	align-self: end;
}
img {
	max-width: 100%;
	max-height: 100%;
}
#submit-button {
	transition: 0.2s linear;
}
#submit-button:hover {
	background-color: var(--beige);
	transition: 0.2s linear;
}
"#);

#[cfg(not(target_family = "wasm"))]
#[component]
pub fn main_page() -> impl IntoView {
	view! {
		<style>{ STYLE }</style>
		<div id="main-content">
			<div id="img-and-overlay">
				<div id="top-left-text">"We're getting married!"</div>
				<img src="/api/assets/main_page_cover.webp"/>
				<div id="bottom-right-text">
					"Maggie Harper & June Welker"
					<br/>
					"December 14, 2024"
				</div>
			</div>
			<div id="second-row">
				<img src="/api/assets/vertical_left_under.webp"/>
				<span id="form-side">
					<img src="/api/assets/gold_flower_corner.webp" id="top-left-corner-deco"/>
					<div id="form-section">
						<h1>"Want an announcement?"</h1>
						<div id="announcement-form">
							<EmailSubmitForm/>
						</div>
					</div>
					<div id="faq-section">
						<h1>"Questions?"</h1>
						<br/>
						<a href="/wedding/faq">"Check the FAQ here!"</a>
					</div>
					<div id="registry-section">
						<h1>"Registry?"</h1>
						<div>"We're working on that :) check the FAQ!"</div>
					</div>
					<img src="/api/assets/gold_flower_corner.webp" id="bottom-right-corner-deco"/>
				</span>
			</div>
		</div>
	}
}

#[island]
fn email_submit_form() -> impl IntoView {

	#[cfg(target_family = "wasm")]
	view!{ }

	#[cfg(not(target_family = "wasm"))]
	{
		//let submit = ServerAction::<AddAnnouncementReq>::new();
		let submit = Action::<AddAnnouncementReq, _>::server();

		view! {
			<div>
				{move || match submit.value().get() {
					None => view! {
						<ActionForm action=submit>
							<div id="form-inputs">
								<label for="name">"name: "</label>
								<input type="text" id="name" name="name" required />
								<label for="address">"address: "</label>
								<input type="text" id="address" name="address" required />
								<label for="email">"email: (in case we need to contact you)"</label>
								<input type="email" id="email" name="email" required />
							</div>
							<input type="submit" value="yes please!" id="submit-button"/>
						</ActionForm>
					// }.into_any(),
					},
					Some(Err(e)) => view! {
						<div>
						{ move || format!("Couldn't submit data: {e}")}
						<br/>
						"Please contact us at junewelker@gmail.com to get this resolved :)"
						</div>
					//}.into_any(),
					}.into_view(),
					Some(Ok(())) => view!{
						<div>
							"Thank you! We'll be sending out announcements soon."
							<br/>
							"In the meantime, if you have anything to let us know, please email us at junewelker@gmail.com"
						</div>
					//}.into_any(),
					}.into_view(),
				}}
			</div>
		}
	}
}
