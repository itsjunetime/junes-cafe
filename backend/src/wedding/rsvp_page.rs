use const_format::concatcp;
use leptos::prelude::*;
use leptos_router::{hooks::use_params, params::{ParamsMap, ParamsError, Params}};
use uuid::Uuid;

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
	#full_address {
		width: 100%;
	}
	textarea {
		resize: vertical;
		width: 100%;
		margin-bottom: 16px;
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
		<style>{ STYLE }</style>
		<Suspense>
			{move || Suspend::new(async move {
				match guest.await {
					Ok(Some(guest)) => {
						view! {
							<h1>{format!("Welcome, {}", &guest.name)}</h1>
							<RsvpForm guest />
						}
							.into_any()
					}
					Ok(None) => {
						view! {
							<div id="initial-response">
								"Oops, looks like that id doesn't exist :/"
							</div>
						}
							.into_any()
					}
					Err(e) => {
						view! {
							<div id="initial-response">
								{move || format!("Couldn't retrieve guest: {e}")}
							</div>
						}
							.into_any()
					}
				}
			})}
		</Suspense>
	}.into_any()
}

#[island]
fn rsvp_form(guest: Guest) -> impl IntoView {
	let submit = ServerAction::<UpdateRsvp>::new();

	// To show:
	// 1. You may bring your family. How many people, total, will be present in your party?
	// 2. You are welcome to bring a +1. Will you be bringing one?
	// 3. Are there any dietary restrictions or facts that we should keep in mind for anyone in
	//    your party?
	// 4. What address should we send your wedding announcement to?
	// 5. Please enter your email address (for time & date information)
	// 6. Please confirm your email address
	let content = move || match submit.value()() {
		None => view! {
			<ActionForm action=submit>
				<label for="full_address">
					"What's your address (including city, state, and country if relevant)? We need this to send you an announcement :)"
				</label>
				<br />
				<input type="text" id="full_address" name="full_address" placeholder="make sure to include city and state :)" required />
				<br />

				<label for="email">"What's your email?"</label>
				<br />
				<input type="email" id="email" name="email" placeholder="email here :)" required />
				<br />

				{move || match guest.party_size {
					PartySize::Group(size) => {
						view! {
							<span id="party-size">
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
							</span>
						}
							.into_any()
					}
					PartySize::NoPlusOne => ().into_any(),
					PartySize::AllowedPlusOne | PartySize::NotBringing | PartySize::Bringing => {
						view! {
							<span id="party-size">
								<label for="accepted_plus_one">
									"Will you be bringing a plus one?"
								</label>
								<br />
								<input
									type="checkbox"
									id="accepted_plus_one"
									name="accepted_plus_one"
									required
									value=guest.party_size == PartySize::Bringing
								/>
							</span>
						}
							.into_any()
					}
				}}
				<br />

				<label for="extra_notes">
					"Are there any dietary restrictions or notes that we should keep in mind for anyone in your party?"
				</label>
				<br />
				<textarea id="extra_notes" name="extra_notes" placeholder="dietary restrictions!" />
				<br />

				<input type="text" id="id" name="id" value=guest.id.to_string() hidden required />

				<input type="submit" value="Submit" />
			</ActionForm>
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
