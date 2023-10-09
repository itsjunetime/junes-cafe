use yew::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
	HtmlButtonElement,
	HtmlInputElement,
	HtmlTextAreaElement,
	DragEvent,
	FileList
};
use super::{
	style::SharedStyle,
	GetPostErr
};
use gloo_net::http::Request;
use gloo_console::log;
use std::{
	collections::HashSet,
	rc::Rc
};

// Since postgres starts ids at 1, we know that no post should have an id of 0, and thus we should
// be safe to use it as the identifier meaning 'No post'
// It irks me to have a sentinel value but since we're communicating over HTTP APIs, it's necessary
// sometimes
pub const NO_POST: u32 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
enum SubmissionState {
	Preparing,
	Loading,
	Resolved(u16, String)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ImageUploadState {
	// If we haven't tried anything yet
	None,
	// If something went wrong before we could even start uploading
	PreflightError(String),
	// If the request to upload is currently pending
	UploadingImage,
	// Result<(id, if_text_inserted), (status_code, error_text)>
	Resolved(Result<(u64, bool), (u16, String)>)
}

#[derive(Debug)]
pub enum EditMsg {
	SetInitial(HashSet<String>, String, String, bool),
	Title(String),
	Content(String),
	AddTag(String),
	RemoveTag(String),
}

#[derive(Properties, PartialEq, Eq, Default, Clone)]
pub struct PostDetails {
	pub id: u32,
	pub title: String,
	pub content: String,
	pub tags: HashSet<String>,
	pub draft: bool
}

impl Reducible for PostDetails {
	type Action = EditMsg;

	// God I fucking hate this. You give me an Rc? And don't make it mutable? Thus removing all the
	// benefits of using an Rc??? Why not just give me the Self itself?? Then I can at least own it
	// and move the values??? I know I'm benefitting off of free work so I don't feel any ire
	// towards those who wrote this, but it still feels like not enough thought was actually given
	// to the API and internal workings, and they just threw `R(ef)?C(ell)?` around everything and
	// called it good. There's gotta be a better way to do this.
	fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
		macro_rules! clone_self{ ($item:ident) => {
			Self { $item, ..(*self).clone() }.into()
		}}

		match action {
			EditMsg::SetInitial(tags, content, title, draft) => Self {
				id: self.id,
				tags,
				content,
				title,
				draft
			}.into(),
			EditMsg::Title(title) => clone_self!(title),
			EditMsg::Content(content) => clone_self!(content),
			EditMsg::AddTag(tag) => if tag.is_empty() {
				self
			} else {
				let mut tags = self.tags.clone();
				tags.insert(tag);

				// And then we want to clear the tag-adder
				if let Some(input) = web_sys::window()
					.and_then(|win| win.document())
					.and_then(|doc| doc.get_element_by_id("new-tag-input"))
					.and_then(|i| i.dyn_into::<HtmlInputElement>().ok()) {

					input.set_value("");
				} else {
					log!("new-tag-input is not actually an input, something is wrong");
				}

				clone_self!(tags)
			},
			EditMsg::RemoveTag(tag) => {
				let mut tags = self.tags.clone();
				tags.remove(&tag);

				clone_self!(tags)
			},
		}
	}
}

#[function_component(EditPostParent)]
pub fn edit_post(props: &super::post::PostProps) -> Html {
	let post = use_state(|| Option::<Result<shared_data::Post, GetPostErr>>::None);
	let details = use_reducer_eq(|| PostDetails { id: props.id, ..PostDetails::default() });

	// These states aren't used until we're showing a post, but it can't be declared conditionally
	// (including after a potential return) or else yew gets angry at us and panics at runtime
	let submit = use_state(|| SubmissionState::Preparing);
	let image = use_state(|| ImageUploadState::None);

	// First we try to retrieve the post if it exists
	{
		let id = props.id.to_owned();
		let state = post.clone();
		use_effect(move || {
			if id != NO_POST && state.is_none() {
				super::get_post(id, state);
			}

			|| ()
		});
	}

	// We only want to do this if we're trying to edit a post and we haven't retrieved it yet
	// And we're assuming that our server-side validation works correctly and ensures that the post
	// that we retrieve isn't allowed to have an empty title
	if props.id != NO_POST && details.title.is_empty() {
		let retrieved_post = match post.as_ref() {
			None => return html! { <p>{ "Checking if post exists to edit..." }</p> },
			Some(Err(GetPostErr::Other(err))) => return html! {
				<p>{ format!("Failed to check if post exists: {err}") }</p>
			},
			Some(Err(GetPostErr::NotFound)) => return html! {
				<p>{ "The post you are trying to edit does not exist" }</p>
			},
			Some(Ok(post)) => post
		};

		// If we do have the post, we won't have returned, so we'll set the details here and let
		// the view reload
		let tags = HashSet::from_iter(retrieved_post.tags.0.clone());
		let content = retrieved_post.orig_markdown.clone();
		let title = retrieved_post.title.clone();

		details.dispatch(EditMsg::SetInitial(tags, content, title, retrieved_post.draft));
	}

	// Prepare things for submitting the post to the backend
	let submit_clone = submit.clone();
	let submit_details = details.clone();
	let id = props.id.to_owned();

	// The callback that will run when we click either the 'Publish' or 'Save as Draft' button,
	// which will create the post, either not as a draft or as a draft (respectively)
	let submit_click = Callback::from(move |event: MouseEvent| {
		// Have to reclone since Callback::from takes an Fn(), not FnOnce()
		let reclone = submit.clone();
		let details_clone = submit_details.clone();
		let owned_id = id.to_owned();

		// Determine if it's a draft by checking the id of the button that clicked it and seeing if
		// that contains the string 'draft'. Kinda stupid and hacky but the easiest way to get this
		// working since we need to use the same callback for publishing and drafting
		let draft = event.target()
			.and_then(|t| t.dyn_into::<HtmlButtonElement>().ok())
			.is_some_and(|t| t.id().to_lowercase().contains("draft"));

		reclone.set(SubmissionState::Loading);

		// Have to spawn a future since we need to await the request's completion
		wasm_bindgen_futures::spawn_local(async move {
			// If we're creating a new post, submit to that endpoint - otherwise, submit to edit
			let url = if owned_id == NO_POST {
				"/api/new_post".into()
			} else {
				format!("/api/edit_post/{owned_id}")
			};

			// Create the data structure to send with the request
			let post_req = shared_data::PostReq {
				title: details_clone.title.clone(),
				content: details_clone.content.clone(),
				tags: Vec::from_iter(details_clone.tags.clone()),
				draft 
			};

			// Create the request
			let (status, text) = match Request::post(&url).json(&post_req) {
				Err(err) => (401, format!("post_req couldn't be serialized: {err:?}")),
				// And then send the request if it's ok
				Ok(req) => match req.send().await {
					Err(err) => (401, format!("gloo_net error: {err:?}")),
					Ok(res) => (res.status(), res.text().await.unwrap_or_else(|e| format!("Couldn't get text: {e:?}")))
				}
			};

			// And set the submittion state with the response so that we can show it
			reclone.set(SubmissionState::Resolved(status, text));
		});
	});

	macro_rules! input_callback{
		($clone:ident, $type:ident, $evtype:ident, $html:ident) => {
			move |e: $evtype| if let Some(msg) = e.target()
				.and_then(|t| t.dyn_into::<$html>().ok())
				.map(|input| EditMsg::$type(input.value())) {
					$clone.dispatch(msg);
				}
		};
		($type:ident) => {{
			let details_clone = details.clone();
			Callback::from(input_callback!(details_clone, $type, Event, HtmlInputElement))
		}}
	}

	// If, at this point, we've uploaded an image, gotten a response, but not yet inserted it into
	// the textarea, we need to do so
	if let ImageUploadState::Resolved(Ok((image_id, false))) = *image {
		let new_text = format!("{}\n\n![Image](/api/images/{image_id})\n\n", details.content);
		details.dispatch(EditMsg::Content(new_text));

		image.set(ImageUploadState::Resolved(Ok((image_id, true))));
	}

	// Get the callbacks for various elements of the editing view
	let title_callback = input_callback!(Title);
	let content_clone = details.clone();
	let content_callback = input_callback!(content_clone, Content, InputEvent, HtmlTextAreaElement);
	let tag_callback = input_callback!(AddTag);

	let image_clone = image.clone();
	let on_image_drop = Callback::from(move |e: DragEvent| {
		e.prevent_default();

		let Some(transfer) = e.data_transfer() else {
			let formatted = "Event's dataTransfer is None".into();
			log!(&formatted);
			image_clone.set(ImageUploadState::PreflightError(formatted));
			return;
		};

		upload_image(transfer.files(), image_clone.clone());
	});

	let input_image = image.clone();

	// Show a different view dependending on if we've submitted the post or not
	match &*submit_clone {
		SubmissionState::Preparing => html! {
			<>
				<SharedStyle />
				<style>
				{
					"
					#article-content {
						max-width: 800px;
						margin: auto;
					}
					#new-tag-input {
						margin-right: 10px;
					}
					span.tag > button {
						background: none;
						color: var(--secondary-text);
						border: none;
						margin-right: -4px;
						margin-left: 4px;
					}
					textarea {
						width: 100%;
						height: 300px;
						resize: none;
					}
					.upload-details {
						border: 1px solid var(--border-color);
						border-radius: 4px;
						padding: 8px 10px;
						width: max-content;
					}
					#submit-button:hover {
						background-color: #00000000;
					}
					#publish-button {
						margin-right: 8px;
					}
					"
				}
				</style>
				<div id="article-content">
					<h1>{ "New Post" }</h1>
					<input placeholder="title" value={ details.title.clone() } onchange={ title_callback } />
					<br/><br/>
					<textarea
						placeholder="what's goin on? :)"
						oninput={ content_callback }
						value={ details.content.clone() }
						disabled={
							matches!(*image, ImageUploadState::UploadingImage | ImageUploadState::Resolved(Ok((_, false))))
						}
					>{
						&details.content
					}</textarea>
					<br/><br/>
					<label
						for="image-upload"
						class="upload-details"
						ondrop={ on_image_drop }
						ondragover={ Callback::from(|e: DragEvent| {
							e.prevent_default();
						}) }
						ondragenter={ Callback::from(|e: DragEvent| {
							e.prevent_default();
						}) }
					>
						{
							match &*image {
								ImageUploadState::None => html! {{ "Drop image here to upload" }},
								ImageUploadState::PreflightError(err) => html! {
									{ format!("Couldn't upload image: {err}") }
								},
								ImageUploadState::UploadingImage => html! {{ "Uploading image..." }},
								ImageUploadState::Resolved(res) => match res {
									Ok((id, inserted)) => html! {{
										if *inserted {
											format!("Image uploaded to id {id}!")
										} else {
											format!("Image uploaded to id {id}, processing...")
										}
									}},
									Err((status, err)) => html! {{
										format!("Image failed to upload with code {status}, error: '{err}'")
									}}
								}
							}
						}
					</label>
					<input
						id="image-upload"
						type="file"
						accept="image/*"
						style="display: none;"
						onchange={ move |e: Event| {
							let ev_type = e.js_typeof();
							let target = e.target();

							// Apparently sometimes we can just get the plain element here or we
							// can get an object of type 'change' and have to grab its target to
							// and then dyn_into that to get the object we want files from
							let input = if let Ok(i) = e.dyn_into::<HtmlInputElement>() {
								i
							} else if let Some(i) = target.and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
								i
							} else {
								let formatted = format!("event was not HtmlInputElement, but rather {ev_type:?}");
								log!(&formatted);
								input_image.set(ImageUploadState::PreflightError(formatted));
								return;
							};

							upload_image(input.files(), input_image.clone());
						}}
					/>
					<br/>
					<div id="tags">
						<h3 id="tag-title">{ "Tags" }</h3>
						<span>
							<input id="new-tag-input" onchange={ tag_callback }/>
							{
								// Show a little tag per tag, with a little button on each to remove
								details.tags.iter().map(|tag| {
									// Ugh I don't like having to do so much re-owning but it's what's necessary
									// I feel like yew doesn't handle lifetimes very well, it just kinda
									// requires re-owning stuff all the time. Especially because `Component`
									// requires a 'static lifetime, so you can't have any borrowed data input
									// structs that impl Component. Makes it all kinda ugly sometimes
									let owned = tag.clone();
									let details_clone = details.clone();

									html! {
										<span class="tag">
											{ tag }
											<button onclick={
												move |_| details_clone.dispatch(EditMsg::RemoveTag(owned.clone()))
											}>{ "✕" }</button>
										</span>
									}
								}).collect::<Html>()
							}
						</span>
					</div>
					<br/><br/>
					<button id="publish-button" onclick={ submit_click.clone() }>{ "Publish" }</button>
					{
						// If this is a completely new post or we're editing a draft, offer the
						// 'save as draft' button. Else don't show it (cause I don't want to figure
						// out the specific mechanics of how to draft already published posts and
						// what happens when you try to un-draft them)
						if details.draft || id == NO_POST {
							html! { <button id="draft-button" onclick={ submit_click }>{ "Save as Draft" }</button> }
						} else {
							html! {}
						}
					}
				</div>
			</>
		},
		SubmissionState::Loading => html! {
			<style>
			{
				"
				body::after {
					content: \"\";
					position: absolute;
					left: 50%;
					top: 50%;
					height:60px;
					width:60px;
					margin:0px auto;
					animation: rotation .6s infinite linear;
					border-left:6px solid rgba(0,174,239,.15);
					border-right:6px solid rgba(0,174,239,.15);
					border-bottom:6px solid rgba(0,174,239,.15);
					border-top:6px solid rgba(0,174,239,.8);
					border-radius:100%;
				}
				@keyframes rotation {
					from {transform: rotate(0deg);}
					to {transform: rotate(359deg);}
				}
				"
			}
			</style>
		},
		SubmissionState::Resolved(code, text) => submit_resolved_view(*code, text, submit_clone.clone())
	}
}

fn submit_resolved_view(code: u16, text: &String, submit_state: UseStateHandle<SubmissionState>) -> Html {
	let go_home = html! {
		<a href="/">{ "Go home" }</a>
	};

	let res_html = if let Some(id) = (code == 200).then(|| text.parse::<u32>().ok()).flatten() {
		html! {
			<div class="submit-result submit-success">
				<h1 class="submit-title">{ "Post was created!" }</h1>
				<div class="nav-buttons">
					<a href={ format!("/post/{id}") }>{ "View Post" }</a>
					<a href={ format!("/edit_post/{id}") }>{ "Edit Post" }</a>
					{ go_home }
				</div>
			</div>
		}
	} else {
		let submit_reason = html! {
			<h3 class="submit-reason">{
				if text.is_empty() {
					"No reason given".into()
				} else {
					format!("Reason given: {text}")
				}
			}</h3>
		};

		match code {
			500.. => html! {
				<div class="submit-result submit-failure">
					<h1 class="submit-title">{ format!("Post creation failed with code {code}") }</h1>
					{ submit_reason }
					<div class="nav-buttons">
						<button onclick={
							Callback::from(move |_| submit_state.set(SubmissionState::Preparing))
						}>{
							"Continue editing"
						}</button>
						{ go_home }
					</div>
				</div>
			},
			_ => html! {
				<div class="submit-result submit-unknown">
					<h1 class="submit-title">{ format!("Request returned with code {code}") }</h1>
					{ submit_reason }
					<div class="nav-buttons">
						{ go_home }
					</div>
				</div>
			}
		}
	};

	html! {
		<>
			<style>
			{
				"
				.submit-result {
					margin: auto;
					width: max-content;
				}
				.submit-success > h1 {
					color: #1a1;
				}
				.submit-failure > h1 {
					color: #a11;
				}
				.submit-unknown {
					color: #55551180;
				}
				.submit-title {
					text-align: center;
				}
				.nav-buttons > a {
					margin: 10px;
					text-decoration: none;
					border: 1px solid var(--border-color);
					border-radius: 4px;
					padding: 6px 10px;
				}
				.nav-buttons > a:hover {
					background-color: var(--main-background);
					transition: all 0.2s;
				}
				"
			}
			</style>
			<SharedStyle />
			{ res_html }
		</>
	}
}

fn upload_image(file_list: Option<FileList>, image: UseStateHandle<ImageUploadState>) {
	macro_rules! fail{
		($reason:expr) => {{
			let formatted = format!($reason);
			log!(&formatted);
			image.set(ImageUploadState::PreflightError(formatted));
			return;
		}};
	}

	let Some(files) = file_list else {
		fail!("DataTransfer's file list is None");
	};

	let len = files.length();
	if len != 1 {
		fail!("Only 1 file can be uploaded at a time ({len} were submitted)");
	}

	let Some(file) = files.item(0) else {
		fail!("files.item(0) returned None despite verifying one existed in the list");
	};

	let file_type = file.type_();
	if !file_type.starts_with("image/") {
		fail!("Uploaded file has bad mime type {file_type}; must be image/*");
	}

	let form = match gloo_net::http::FormData::new() {
		Ok(f) => f,
		Err(err) => fail!("Couldn't create new FormData: {err:?}"),
	};

	if let Err(err) = form.append_with_blob("file", &file) {
		fail!("Couldn't appending blob to form: {err:?}");
	};

	image.set(ImageUploadState::UploadingImage);

	wasm_bindgen_futures::spawn_local(async move {
		let result = match Request::post("/api/post_image").body(form).send().await {
			Err(err) => Err((401, format!("{err:?}"))),
			// If it didn't fail, get the text
			Ok(res) => res.text().await
				// And if we can't get the text, report so
				.map_err(|e| (res.status(), format!("Couldn't get res text: {e:?}")))
				// If we can get the text, see if it returned an OK code
				.and_then(|t| if res.ok() {
					// If it did, see if we can parse it to an ID to display
					t.parse::<u64>()
						// If we can, then map it to the id and `false`, indicating that the ID
						// hasn't been inserted into the textarea as a markdown image yet
						.map(|n| (n, false))
						// If we can't, map it to an error with the status and explanation
						.map_err(|e| (res.status(), format!("'{t}' could not be parsed to u64: {e:?}")))
				} else {
					// And it the response doesn't have an OK status, then just report the status
					// and text back to the UI
					Err((res.status(), t))
				})
		};

		image.set(ImageUploadState::Resolved(result));
	});
}
