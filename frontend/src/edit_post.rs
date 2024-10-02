use yew::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
	HtmlButtonElement,
	HtmlInputElement,
	HtmlTextAreaElement,
	FileList,
	FormData
};
use std::str::FromStr;
use super::{
	style::SharedStyle,
	GetPostErr
};
use gloo_net::http::Request;
use gloo_console::log;
use gloo_timers::future::TimeoutFuture;
use std::{
	collections::HashSet,
	rc::Rc,
	cell::RefCell
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
enum AssetUploadState {
	// If we haven't tried anything yet
	None,
	// If something went wrong before we could even start uploading
	PreflightError(String),
	// If the request to upload is currently pending
	Uploading,
	// Result<(id, if_text_inserted), (status_code, error_text)>
	Resolved(Result<(String, bool), (u16, String)>)
}

#[derive(Debug)]
pub enum EditMsg {
	SetInitial(HashSet<String>, String, String, bool),
	Title(String),
	Content(String),
	RenderedContent(String),
	AddTag(String),
	RemoveTag(String),
}

#[derive(Properties, PartialEq, Eq, Default, Clone)]
pub struct PostDetails {
	pub id: u32,
	pub title: String,
	pub content: String,
	pub rendered_content: String,
	pub tags: HashSet<String>,
	pub draft: bool
}

impl Reducible for PostDetails {
	type Action = EditMsg;

	fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
		macro_rules! clone_self{ ($item:ident$(, $other:ident)*) => {
			Self { $item, $($other, )*..(*self).clone() }.into()
		}}

		match action {
			EditMsg::SetInitial(tags, content, title, draft) => {
				let rendered_content = shared_data::md_to_html(&content);
				Self {
					id: self.id,
					tags,
					content,
					rendered_content,
					title,
					draft
				}.into()
			},
			EditMsg::Title(title) => clone_self!(title),
			EditMsg::Content(content) => clone_self!(content),
			EditMsg::RenderedContent(rendered_content) => clone_self!(rendered_content),
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

#[derive(Properties, PartialEq)]
pub struct PostProps {
	pub id: u32
}

#[function_component(EditPostParent)]
pub fn edit_post(props: &PostProps) -> Html {
	let post = use_state(|| Option::<Result<shared_data::Post, GetPostErr>>::None);
	let details = use_reducer_eq(|| PostDetails { id: props.id, ..PostDetails::default() });
	// kinda hate storing a Rc<RefCell> inside use_state but whatever, it seems to be the only way
	// to actually keep a consistent reference to something inside this function and a promise at
	// the same time
	let render_uuid = use_state(|| Rc::new(RefCell::new(uuid::Uuid::new_v4())));

	// These states aren't used until we're showing a post, but it can't be declared conditionally
	// (including after a potential return) or else yew gets angry at us and panics at runtime
	let submit = use_state(|| SubmissionState::Preparing);
	let asset = use_state(|| AssetUploadState::None);

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

	fn set_text(
		text: String,
		render: &UseStateHandle<Rc<RefCell<uuid::Uuid>>>,
		details: &UseReducerHandle<PostDetails>
	) {
		let text_clone = text.clone();

		let new_uuid = ::uuid::Uuid::new_v4();
		*render.borrow_mut() = new_uuid;
		let render = render.clone();
		let details = details.clone();

		details.dispatch(EditMsg::Content(text));

		wasm_bindgen_futures::spawn_local(async move {
			TimeoutFuture::new((text_clone.len() / 100) as u32).await;

			if *render.borrow() != new_uuid { return; }

			let rendered = shared_data::md_to_html(&text_clone);
			details.dispatch(EditMsg::RenderedContent(rendered));
		});
	}

	// Prepare things for submitting the post to the backend
	let submit_clone = submit.clone();
	let submit_details = details.clone();
	let id = props.id;

	// The callback that will run when we click either the 'Publish' or 'Save as Draft' button,
	// which will create the post, either not as a draft or as a draft (respectively)
	let submit_click = Callback::from(move |event: MouseEvent| {
		// Have to reclone since Callback::from takes an Fn(), not FnOnce()
		let reclone = submit.clone();
		let details_clone = submit_details.clone();

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
			let url = if id == NO_POST {
				"/api/new_post".into()
			} else {
				format!("/api/edit_post/{id}")
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

	// If, at this point, we've uploaded an asset, gotten a response, but not yet inserted it into
	// the textarea, we need to do so
	if let AssetUploadState::Resolved(Ok((ref asset_id, false))) = *asset {
		let is_image = asset_id.split('.')
			.last()
			.is_some_and(|ext| ["jpeg", "jpg", "png", "webp", "heic", "heif"].contains(&ext));

		let exclamation = if is_image { "!" } else { "" };
		let new_text = format!("{}\n\n{exclamation}[Asset](/api/assets/{asset_id})\n\n", details.content);

		asset.set(AssetUploadState::Resolved(Ok((asset_id.to_string(), true))));
		set_text(new_text, &render_uuid, &details);
	}

	macro_rules! input_callback{
		($type:ident) => {{
			let details_clone = details.clone();
			Callback::from(move |e: Event| if let Some(msg) = e.target()
				.and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
				.map(|input| EditMsg::$type(input.value())) {
					details_clone.dispatch(msg);
				}
			)
		}}
	}

	// Get the callbacks for various elements of the editing view
	let content_clone = details.clone();
	let render_clone = render_uuid.clone();
	let content_callback = move |e: InputEvent| if let Some(input) = e.target()
		.and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok()) {
			let new_text = input.value();
			set_text(new_text, &render_clone, &content_clone);

			let style = input.style();
			if let Err(e) = style.set_property("height", "auto") {
				log!("Couldn't set height to auto: ", e);
			}
			let new_height = format!("{}px", input.scroll_height());
			if let Err(e) = style.set_property("height", new_height.as_str()) {
				log!("Couldn't update height correctly: ", e);
			}
		};
	let title_callback = input_callback!(Title);
	let tag_callback = input_callback!(AddTag);

	let asset_clone = asset.clone();
	let on_asset_drop = Callback::from(move |e: DragEvent| {
		e.prevent_default();

		let Some(transfer) = e.data_transfer() else {
			let formatted = "Event's dataTransfer is None".into();
			log!(&formatted);
			asset_clone.set(AssetUploadState::PreflightError(formatted));
			return;
		};

		upload_asset(transfer.files(), asset_clone.clone());
	});

	let input_asset = asset.clone();

	// Show a different view dependending on if we've submitted the post or not
	match &*submit_clone {
		SubmissionState::Preparing => html! {
			<>
				<SharedStyle />
				<style>
				{
					"
					#article-content {
						margin: auto;
					}
					#new-tag-input {
						margin-right: 10px;
					}
					span.tag > button {
						background: none;
						color: var(--secondary-text);
						border: none;
						margin-left: 8px;
						padding: 0;
					}
					#tags span.tag {
						padding: 3px 6px 4px 6px;
					}
					#edit-and-render {
						display: grid;
						grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
						column-gap: 20px;
						max-width: 140%;
					}
					textarea {
						height: 300px;
						resize: vertical;
					}
					#rendered img {
						max-width: 100%;
					}
					#rendered {
						border: 1px solid var(--border-color);
						border-radius: 8px;
						padding: 0px 20px;
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
					<div id="edit-and-render">
						<textarea
							placeholder="what's goin on? :)"
							oninput={ content_callback }
							value={ details.content.clone() }
							disabled={
								matches!(*asset, AssetUploadState::Uploading | AssetUploadState::Resolved(Ok((_, false))))
							}
						>{
							&details.content
						}</textarea>
						<span id="rendered">{
							match AttrValue::from_str(details.rendered_content.as_str()) {
								Ok(s) => Html::from_html_unchecked(s),
							}
						}</span>
					</div>
					<br/><br/>
					<label
						for="asset-upload"
						class="upload-details"
						ondrop={ on_asset_drop }
						ondragover={ Callback::from(|e: DragEvent| e.prevent_default()) }
						ondragenter={ Callback::from(|e: DragEvent| e.prevent_default()) }
					>
						{
							match &*asset {
								AssetUploadState::None => html! {{ "Drop asset here to upload" }},
								AssetUploadState::PreflightError(err) => html! {
									{ format!("Couldn't upload asset: {err}") }
								},
								AssetUploadState::Uploading => html! {{ "Uploading asset..." }},
								AssetUploadState::Resolved(res) => match res {
									Ok((id, inserted)) => html! {{
										if *inserted {
											format!("Asset uploaded to id {id}!")
										} else {
											format!("Asset uploaded to id {id}, processing...")
										}
									}},
									Err((status, err)) => html! {{
										format!("Asset failed to upload with code {status}, error: '{err}'")
									}}
								}
							}
						}
					</label>
					<input
						id="asset-upload"
						type="file"
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
								input_asset.set(AssetUploadState::PreflightError(formatted));
								return;
							};

							upload_asset(input.files(), input_asset.clone());
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
		SubmissionState::Resolved(code, text) => submit_resolved_view(*code, text, props.id, submit_clone.clone())
	}
}

fn submit_resolved_view(code: u16, text: &String, post_id: u32, submit_state: UseStateHandle<SubmissionState>) -> Html {
	let go_home = html! {
		<a href="/">{ "Go home" }</a>
	};

	let id = (code == 200).then(|| text.parse::<u32>().ok())
		.flatten()
		.or(if post_id == NO_POST { None } else { Some(post_id) });

	let res_html = if let Some(id) = id {
		// If we're editing a post...
		let creation_str = if post_id == NO_POST {
			"Post was saved!"
		} else {
			"Post was created!"
		};
		html! {
			<div class="submit-result submit-success">
				<h1 class="submit-title">{ creation_str }</h1>
				<div class="nav-buttons">
					<a href={ format!("/post/{id}") }>{ "View Post" }</a>
					<a href={ format!("/admin/edit_post/{id}") }>{ "Edit Post" }</a>
					{ go_home }
				</div>
			</div>
		}
	} else {
		let bottom_section = html! { <>
			<h3 class="submit-reason">{
				if text.is_empty() {
					"No reason given".into()
				} else {
					format!("Reason given: {text}")
				}
			}</h3>
			<div class="nav-buttons">
				<button onclick={
					Callback::from(move |_| submit_state.set(SubmissionState::Preparing))
				}>{
					"Continue editing"
				}</button>
				{ go_home }
			</div>
		</> };

		match code {
			500.. => html! {
				<div class="submit-result submit-failure">
					<h1 class="submit-title">{ format!("Post creation failed with code {code}") }</h1>
					{ bottom_section }
				</div>
			},
			_ => html! {
				<div class="submit-result submit-unknown">
					<h1 class="submit-title">{ format!("Request returned with code {code}") }</h1>
					{ bottom_section }
				</div>
			}
		}
	};

	html! {
		<>
			<SharedStyle />
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
			{ res_html }
		</>
	}
}

fn upload_asset(file_list: Option<FileList>, asset: UseStateHandle<AssetUploadState>) {
	macro_rules! fail{
		($reason:expr) => {{
			let formatted = format!($reason);
			log!(&formatted);
			asset.set(AssetUploadState::PreflightError(formatted));
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


	let form = match FormData::new() {
		Ok(f) => f,
		Err(err) => fail!("Couldn't create new FormData: {err:?}"),
	};

	let name = file.name();

	if let Err(err) = form.append_with_str("name", &name) {
		fail!("Couldn't append name to form: {err:?}");
	}

	if let Err(err) = form.append_with_blob("file", &file) {
		fail!("Couldn't append blob to form: {err:?}");
	};

	asset.set(AssetUploadState::Uploading);

	wasm_bindgen_futures::spawn_local(async move {
		let request = match Request::post("/api/post_asset").body(form) {
			Ok(rq) => rq,
			Err(e) => fail!("Couldn't create request: {e}"),
		};

		let result = match request.send().await {
			Err(err) => Err((401, format!("{err:?}"))),
			// If it didn't fail, get the text
			Ok(res) => res.text().await
				// And if we can't get the text, report so
				.map_err(|e| (res.status(), format!("Couldn't get res text: {e:?}")))
				// If we can get the text, see if it returned an OK code
				.and_then(|t| if res.ok() {
					Ok((t, false))
				} else {
					// And it the response doesn't have an OK status, then just report the status
					// and text back to the UI
					Err((res.status(), t))
				})
		};

		asset.set(AssetUploadState::Resolved(result));
	});
}
