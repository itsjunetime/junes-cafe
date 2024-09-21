use const_format::concatcp;
use leptos::prelude::*;
use leptos_router::{hooks::use_params, params::{ParamsMap, ParamsError, Params}};
use leptos_meta::Title;
use uuid::Uuid;
use web_sys::{FormData, HtmlFormElement, HtmlInputElement, SubmitEvent};
use wasm_bindgen::JsCast;

use super::{SHARED_READABLE, server::{guest_with_id, Guest, UpdateRsvp, PartySize}};

use std::{str::FromStr, sync::Arc};

#[derive(PartialEq, Clone)]
struct UserId(Uuid);

impl Params for UserId {
	fn from_map(map: &ParamsMap) -> Result<Self, ParamsError> {
		map.get_str("id")
			.ok_or_else(|| ParamsError::MissingParam("id".into()))
			.and_then(|s|
				Uuid::from_str(s)
					.map_err(|e| ParamsError::Params(Arc::new(e)))
			)
			.map(Self)
	}
}

const STYLE: &str = concatcp!(
	SHARED_READABLE,
	r#"
	#full_address, #email {
		width: 100%;
	}
	textarea {
		resize: vertical;
		width: 100%;
		margin-bottom: 16px;
	}
	#checkbox-details {
		margin-left: 20px;
	}
	.sublabel, .sublabel * {
		font-size: 16px;
	}
	#err_title {
		color: red;
	}
	h3 {
		margin: 0;
	}
	#faq-suggestion {
		margin-top: 16px;
	}
	"#
);

#[component]
pub fn rsvp_page() -> impl IntoView {
	let Ok(UserId(user_id)) = use_params::<UserId>().get() else {
		return view! { "Please provide an id to work with (e.g. /wedding/rsvp/{id})" }.into_any()
	};

	let guest = Resource::new(|| (), move |()| guest_with_id(user_id));

	view! {
		<Title text="Harper/Welker Wedding RSVP"/>
		<style>{ STYLE }</style>
		<Suspense>
			{move || Suspend::new(async move {
				match guest.await {
					Ok(Some(guest)) => view!{
						<Title text="Harper/Welker Wedding RSVP form"/>
						<h1>{format!("Welcome, {}", &guest.name)}</h1>
						<RsvpForm guest />
					}.into_any(),

					Ok(None) => view! {
						<div id="initial-response">
							"Oops, looks like that id doesn't exist :/"
						</div>
					}.into_any(),

					Err(e) => view! {
						<div id="initial-response">
							{move || format!("Couldn't retrieve guest: {e}")}
						</div>
					}.into_any()
				}
			})}
		</Suspense>
		<div id="faq-suggestion">
			<h3>"Questions?"</h3>
			<span>"Check out the "<a href="/wedding/faq">"FAQ!"</a></span>
		</div>
	}.into_any()
}

#[island]
fn rsvp_form(guest: Guest) -> impl IntoView {
	let submit = ServerAction::<UpdateRsvp>::new();
	let (err, set_err) = signal(None);
	let (attending, set_attending) = signal(true);

	let submit_callback = move |ev: SubmitEvent| {
		ev.prevent_default();
		let form = ev.target()
			.expect("Something submitted this")
			.dyn_into::<HtmlFormElement>()
			.expect("This is what it must be");

		let data = FormData::new_with_form(&form)
			.expect("this just gotta work");

		let doc = window()
			.document()
			.expect("No document???");

		let attending = doc.query_selector("#attending")
			.ok()
			.flatten()
			.expect("this checkbox better be there")
			.dyn_ref::<HtmlInputElement>()
			// yes we know it's an input
			.unwrap()
			.checked()
			.to_string();

		data.set_with_str("attending", &attending.to_string())
			.unwrap();

		let accepted_plus_one = doc.query_selector("#accepted_plus_one")
			.expect("This better not error")
			.and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
			.map(|input| input.checked());

		if let Some(accepted) = accepted_plus_one {
			data.set_with_str("accepted_plus_one", &accepted.to_string())
				.unwrap();
		}

		match UpdateRsvp::from_form_data(&data) {
			Err(e) => {
				set_err(Some(format!("Couldn't create update request: {e}")));
				ev.prevent_default();
			},
			Ok(update) => drop(submit.dispatch(update)),
		}
	};

	// To show:
	// 1. You may bring your family. How many people, total, will be present in your party?
	// 2. You are welcome to bring a +1. Will you be bringing one?
	// 3. Are there any dietary restrictions or facts that we should keep in mind for anyone in
	//    your party?
	// 4. What address should we send your wedding announcement to?
	// 5. Please enter your email address (for time & date information)
	// 6. Please confirm your email address
	let content = move || match submit.value()() {
		None if guest.party_size == PartySize::NotAttending => view!{
			<div>
				"You have previously indicated you are not able to attend our wedding celebration. If that is not the case anymore, please contact us personally :)"
			</div>
		}.into_any(),
		None => view! {
			<form on:submit=submit_callback>
				<label for="full_address">
					<span>"What's your address (including city, state, and country if relevant)?"</span>
					<div class="sublabel">"We need this to send you an announcement :)"</div>
				</label>
				<input
					type="text"
					id="full_address"
					name="full_address"
					placeholder="make sure to include city and state!"
					value={ guest.full_address.clone() }
					required
				/>
				<br />

				<label for="email">"What's your email?"</label>
				<br />
				<input type="email" id="email" name="email" placeholder="email here :)" value={ guest.email.clone() } required/>
				<br />

				<input
					type="checkbox"
					id="attending"
					name="attending"
					prop:checked={ attending() }
					on:input={move |ev| set_attending(
							ev.target()
								.expect("this has to have a target")
								.dyn_into::<HtmlInputElement>()
								.expect("that's what it has to be")
								.checked()
						)
					}
				/><span>"I can attend"</span>
				<div id="checkbox-details" class="sublabel">
					<label for="attending">
						"^ Check this box if you are able to attend our wedding celebration on the evening of December 14, 2024, in SLC, Utah."
					</label>
					<br/>
					<span>"(Details about specific times and addresses will be emailed to you if so)"</span>
				</div>


				{if attending() {
					view!{
						{ match guest.party_size {
							PartySize::NoPlusOne | PartySize::NotAttending => ().into_any(),
							PartySize::Group(size) => view! {
								<div id="party-size">
									<label for="group_size">
										"How many people will be in your party, total?"
									</label>
									<br />
									<input
										type="number"
										id="group_size"
										name="group_size"
										min="1"
										required
										value=size
									/>
								</div>
							}.into_any(),
							PartySize::AllowedPlusOne | PartySize::NotBringing | PartySize::Bringing => {
								view! {
									<div id="party-size">
										<input
											type="checkbox"
											id="accepted_plus_one"
											name="accepted_plus_one"
											prop:checked={ guest.party_size == PartySize::Bringing }
										/>
										<label for="accepted_plus_one">
											"Check this if you will be bringing a plus-one"
										</label>
									</div>
								}.into_any()
							}
						}}

						<label for="extra_notes">
							"Are there any dietary restrictions or notes that we should keep in mind for anyone in your party?"
						</label>
						<br />
						<textarea
							id="extra_notes"
							name="extra_notes"
							placeholder="dietary restrictions?"
							prop:value={ guest.extra_notes.clone() }
						/>
						<br />
					}.into_any()
				} else {
					view!{
						<input type="text" value="" style="display: none;" name="extra_notes" id="extra_notes"/>
					}.into_any()
				}}

				<input type="text" id="id" name="id" value=guest.id.to_string() hidden required />

				<input type="submit" value="Submit" on:submit=submit_callback />

				{ move || match err() {
					Some(err) => view! {
						<div><span id="err_title">"error: "</span>{ err }</div>
					}.into_any(),
					None => ().into_any()
				}}
			</form>
		}.into_any(),
		Some(Err(e)) => view! {
			<div id="form-response">
				{move || format!("Couldn't submit response: {e}")} <br />
				"Your best bet is to try again later or contact us at junewelker@gmail.com. Sorry again :/"
			</div>
		}.into_any(),
		Some(Ok(())) => view! {
			<div id="form-response">
				"Thank you! We'll email you once there's more details to share :)"
			</div>
		}.into_any(),
	};

	view! { <div>{content}</div> }
}
