use leptos::{web_sys::window, prelude::*};
use super::{view_with_title, SHARED_READABLE, server::{all_relations, AddGuest, Relation, PartySize, NOT_AUTHORIZED_ERR}};

// unfortunately, this whole thing's gotta be an island 'cause we want the list of relations to be
// reactive to when we add a new one
#[island]
pub fn admin() -> impl IntoView {
	let new_guest = ServerMultiAction::<AddGuest>::new();
	let relations = Resource::new(move || new_guest.version(), move |_| all_relations());

	view_with_title("Wedding Admin", view! {
		<style>{ SHARED_READABLE }</style>
		<style>
			{
				"form {
					margin-bottom: 20px;
				}
				details > :not(summary) {
					margin-left: 20px;
				}
				details > :not(summary) * {
					font-size: 18px;
				}
				summary > em {
					font-size: 18px;
				}
				details > button {
					padding: 0 10px;
				}
				input, select, label {
					margin-right: 10px;
				}"
			}
		</style>
		<Suspense>
			{move || Suspend::new(async move {
				// leptos. why do i have to do this. I think the trait system is being fucky 'cause
				// `relations` impls IntoFuture. And rust admits that. But won't compile when I
				// just try to await it. Who knows

				let res = relations.by_ref().await;
				{
					// this `move` is necessary to make leptos render correctly - something about the
					// owning/tracking system or whatever.
					move || match *res {
						Err(ServerFnError::ServerError(ref err)) if err == NOT_AUTHORIZED_ERR => {
							view! {
								<!DOCTYPE html>
								<html>
									<head>
										// just redirect them to the normal admin since that has the yew interactive login
										// thing
										// mmm do also we want to do a ref= thing with the login? to redirect to the right
										// path? hmm
										<meta http-equiv="refresh" content="0; url=/admin" />
									</head>
								</html>
							}
								.into_any()
						}
						Err(ref e) => view! {
							<div>{format!("Ran into an error: {e}")}</div>
						}.into_any(),
						Ok(ref relations) => {
							// feels kinda bad to clone but if we could `await` `relations` itself, then it
							// would be cloned away, so this isn't like a performance hit
							let guests = relations
								.iter()
								.flat_map(|r| match r {
									Relation::Invitee(g) => Some(g.clone()),
									_ => None,
								});
							let recips = relations
								.iter()
								.flat_map(|r| match r {
									Relation::AnnouncementOnly(r) => Some(r.clone()),
									_ => None,
								});

							view! {
								<h1>"Guests"</h1>
								// todo: make a little table up here about total rsvp'd size, total
								// invited size, estimated size accounting for current rsvp rate,
								// etc.
								{guests
									.map(|g| {
										view! {
											<details>
												<summary>
													{g.name}
													" "
													<em>{g.email.unwrap_or_else(|| "no email".into())}</em>
													<br />
												</summary>
												<div><strong>"Party Size: "</strong>{g.party_size.to_string()}</div>
												<div>
													<strong>"Extra notes: "</strong>
													{g.extra_notes.unwrap_or_else(|| "no notes".into())}
												</div>
												<div><strong>"UUID: "</strong>{g.id.to_string()}</div>
												<button on:click=move |_| copy_rsvp_link_to_clipboard(g.id)>
													<em>"Copy link"</em>
												</button>
											</details>
										}
									})
									.collect_view()}
								<h1>"Announcement Recipients"</h1>
								<table>
									<tr>
										<th>"Name"</th>
										<th>"Address"</th>
										<th>"Email"</th>
									</tr>
									{recips
										.map(|recip| {
											view! {
												<tr>
													<td>{recip.name}</td>
													<td>{recip.address}</td>
													<td>{recip.email}</td>
												</tr>
											}
										})
										.collect_view()}
								</table>
								<h1>"New Guest"</h1>

								<MultiActionForm action=new_guest>
									<div id="form-inputs">
										<label for="name">"Name: "</label>
										<input type="text" id="name" name="name" required />
										<label for="party_size">"Party Size:"</label>
										<select id="party_size" name="party_size">
											<option value=PartySize::SELECT_GROUP>
												{PartySize::SELECT_GROUP}
											</option>
											<option value=PartySize::SELECT_NO_PLUS_ONE>
												{PartySize::SELECT_NO_PLUS_ONE}
											</option>
											<option value=PartySize::SELECT_PLUS_ONE>
												{PartySize::SELECT_PLUS_ONE}
											</option>
										</select>
										<input type="submit" value="Submit" />
									</div>
								</MultiActionForm>
							}
								.into_any()
						}
					}
				}
			})}
		</Suspense>
	})
}

fn copy_rsvp_link_to_clipboard(id: uuid::Uuid) {
	wasm_bindgen_futures::spawn_local(async move {
		let url = format!("https://itsjuneti.me/wedding/rsvp/{}", id);
		let promise = window().expect("No window??")
			.navigator()
			.clipboard()
			.write_text(&url);

		wasm_bindgen_futures::JsFuture::from(promise)
			.await
			.expect("Couldn't write link to clipboard");
	});
}
