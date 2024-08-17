use leptos::prelude::*;

use const_format::concatcp;
use super::server::AddAnnouncementReq;

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
	padding: 4px 12px 3px 12px;
}
form > input, #form-side > div > * {
	margin-left: 50%;
	transform: translateX(-50%);
	display: inline-block;
	width: max-content;
}
#form-inputs {
	margin-bottom: 6px;
}
#form-inputs > input {
	margin-top: 4px;
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
	top: 4vw;
	left: 8vw;
	font-size: 4vw;
}
#bottom-right-text {
	bottom: 10px;
	right: 10px;
	color: var(--white);
	text-align: end;
	text-shadow: 0px 0px 14px rgba(0, 0, 0, 0.9);
	font-size: 4vw;
}
#img-and-overlay > div {
	z-index: 10;
	position: absolute;
	width: max-content;
}
#second-row {
	display: grid;
	margin: 1vw;
}
@media screen and (min-width: 500px) {
	#second-row {
		grid-template-columns: 2fr 3fr;
	}
}
@media screen and (max-width: 500px) {
	#second-row {
		grid-template-rows: 1fr 1fr;
		grid-row-gap: 20px;
	}
	#second-row > img {
		order: 1;
	}
}
#second-row > img {
	border-radius: 12px;
	max-width: 100%;
}
#form-side {
	display: grid;
	grid-template-rows: auto auto auto;
	font-size: 2vh;
	margin: 2vw;
}
#form-side > div {
	margin: auto 0;
}
#form-side > img {
	position: absolute;
	width: 12vw;
}
#form-section > h1 {
	text-align: center;
}
div#form-section {
	margin-top: 4vw;
}
#bottom-right-corner-deco {
	rotate: 180deg;
	right: 36px;
	align-self: end;margin
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
#form-response {
	text-align: center;
}
div#registry-section {
	margin-bottom: 6vw;
}
announcement-form {
	min-width: 60%;
}
"#);

#[component]
pub fn main_page() -> impl IntoView {
	view! {
		<style>{STYLE}</style>
		<div id="main-content">
			<div id="img-and-overlay">
				<div id="top-left-text">"We're getting married!"</div>
				<img src="/api/assets/main_page_cover.webp" />
				<div id="bottom-right-text">
					"Maggie Harper & June Welker" <br /> "December 14, 2024"
				</div>
			</div>
			<div id="second-row">
				<img src="/api/assets/vertical_left_under.webp" />
				<span id="form-side">
					<img src="/api/assets/gold_flower_corner.webp" id="top-left-corner-deco" />
					<div id="form-section">
						<h1>"Want an announcement?"</h1>
						<div id="announcement-form">
							<EmailSubmitForm />
						</div>
					</div>
					<div id="faq-section">
						<h1>"Questions?"</h1>
						<br />
						<a href="/wedding/faq">"Check the FAQ here!"</a>
					</div>
					<div id="registry-section">
						<h1>"Registry?"</h1>
						<div>
							"Yes! "
							<a href="https://www.zola.com/registry/maggieandjune">
								"Just click here"
							</a>
						</div>
					</div>
					<img src="/api/assets/gold_flower_corner.webp" id="bottom-right-corner-deco" />
				</span>
			</div>
		</div>
	}
}

#[island]
fn email_submit_form() -> impl IntoView {
	let submit = ServerAction::<AddAnnouncementReq>::new();

	let content = move || match submit.value()() {
		None => view! {
			<ActionForm action=submit>
				<div id="form-inputs">
					<label for="name">"name: "</label>
					<br />
					<input type="text" id="name" name="name" required />
					<br />
					<label for="address">"address: "</label>
					<br />
					<input type="text" id="address" name="address" required />
					<br />
					<label for="email">"email: (in case we need to contact you)"</label>
					<br />
					<input type="email" id="email" name="email" required />
					<br />
				</div>
				<input type="submit" value="yes please!" id="submit-button" />
			</ActionForm>
		}.into_any(),
		Some(Err(e)) => view! {
			<div id="form-response">
				{move || format!("Couldn't submit data: {e}")} <br />
				"Please contact us at junewelker@gmail.com to get this resolved :)"
			</div>
		}.into_any(),
		Some(Ok(())) => view! {
			<div id="form-response">
				"Thank you! We'll be sending out announcements soon." <br />
				"In the meantime, if you have anything to let us know, please email us at junewelker@gmail.com"
			</div>
		}.into_any(),
	};

	view! { <div>{content}</div> }
}
