use std::{borrow::Cow, collections::HashSet, fmt::Write};

use leptos::{either::Either, prelude::*, wasm_bindgen::JsCast, task::spawn_local};
use serde::{Deserialize, Serialize};
use server_fn::codec::{MultipartData, Json};
use web_sys::{FileList, FormData, HtmlTextAreaElement};
use gloo_console::log;
use shared_data::{BASE_STYLE, md_to_html, PostReq};

#[cfg(not(target_family = "wasm"))]
mod ssr {
	use axum::{extract::{FromRef, FromRequestParts, Path, State}, response::{IntoResponse, Response}, RequestExt};
	use backend::{auth::get_username, AxumState};
	use http::{request::Parts, HeaderName, HeaderValue, Request, StatusCode};
	use leptos_axum::ResponseOptions;
	use leptos::{either::Either, prelude::*};
	use sqlx::{Pool, Postgres};
	use tower_sessions::Session;

	use std::borrow::Cow;

	use crate::pages::RedirLocation;
	use super::{EditPost, PostDetails, super::Redirect};

	pub async fn new_post(
		session: Session,
		State(state): State<AxumState>,
		req: Request<axum::body::Body>
	) -> Response {
		edit_post_handler_inner(req, state, session, None).await
	}

	pub async fn edit_post_handler(
		session: Session,
		State(state): State<AxumState>,
		mut req: Request<axum::body::Body>
	) -> Response {
		let mut parts: Parts = req.extract_parts().await.unwrap();
		let Ok(Path(path)) = Path::<u32>::from_request_parts(&mut parts, &state).await else {
			return "You need to provide a path".into_response()
		};

		edit_post_handler_inner(req, state, session, Some(path)).await
	}

	pub async fn edit_post_handler_inner(
		req: Request<axum::body::Body>,
		state: AxumState,
		session: Session,
		path: Option<u32>,
	) -> Response {
		let handler = leptos_axum::render_app_async(move || {
			let session = session.clone();
			let state = state.clone();
			Suspend::new(async move {
				let pool = Pool::from_ref(&state.tx_state);

				edit_post_shell(
					session,
					LeptosOptions::from_ref(&state),
					pool,
					path
				).await
			})
		});

		handler(req).await.into_response()
	}

	pub async fn edit_post_shell(
		session: Session,
		options: LeptosOptions,
		pool: Pool<Postgres>,
		id: Option<u32>
	) -> Either<impl IntoView, impl IntoView> {
		let Some(username) = get_username(&session).await else {
			let url = Redirect {
				force_login: true,
				redir_to: id.map_or(RedirLocation::NewPost, RedirLocation::EditPost)
			}.url().into_owned();

			let resp: ResponseOptions = expect_context();
			resp.set_status(StatusCode::TEMPORARY_REDIRECT);
			resp.append_header(HeaderName::from_static("location"), HeaderValue::from_str(url.as_ref()).unwrap());

			return Either::Right(view!{
				<!DOCTYPE html>
				<html lang="en">
					<head>
						<HydrationScripts options/>
					</head>
					<body>
						<a href={ url }>"You need to log in"</a>
					</body>
				</html>
			})
		};

		let post = match id {
			Some(id) => match crate::blog_api::get_post_for_user(&pool, id, Some(username)).await {
				Ok(p) => Either::Left(PostDetails {
					id: Some(p.id),
					title: p.title,
					content: p.orig_markdown,
					rendered_content: p.html,
					tags: p.tags.0.into_iter().collect(),
					draft: p.draft
				}),
				// TODO: How to handle these errors? We need to tell the user but leptos' return types
				// are strongly typed depending on the content of the page so we can't just return
				// early
				Err(sqlx::Error::RowNotFound) => Either::Right(Cow::Borrowed("No such post found!")),
				Err(e) => Either::Right(format!("Couldn't get post: {e}").into())
			},
			None => Either::Left(PostDetails { id: None, ..PostDetails::default() })
		};

		Either::Left(view!{
			<!DOCTYPE html>
			<html lang="en">
				<head>
					<meta charset="utf-8"/>
					<meta name="viewport" content="width=device-width, initial-scale=1"/>
					<AutoReload options=options.clone() />
					<HydrationScripts options islands=true/>
				</head>
				<body>{
					match post {
						Either::Left(post) => Either::Left(view!{ <EditPost post=post/> }),
						Either::Right(error) => Either::Right(view!{{ error }})
					}
				}</body>
			</html>
		})
	}
}

#[cfg(not(target_family = "wasm"))]
pub use ssr::*;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct PostDetails {
	pub id: Option<u32>,
	pub title: String,
	pub content: String,
	pub rendered_content: String,
	pub tags: HashSet<String>,
	pub draft: bool
}

#[derive(PartialEq)]
enum AssetUploadState {
	// If we haven't tried anything yet
	None,
	// If something went wrong before we could even start uploading
	PreflightError(Cow<'static, str>),
	// If the request to upload is currently pending
	Uploading,
	// Result<(id, if_text_inserted), (status_code, error_text)>
	Resolved(Result<(String, bool), (u16, String)>)
}

enum SubmissionState {
	Preparing,
	Loading,
	Resolved(Result<String, String>)
}

const POST_BODY_INPUT_ID: &str = "post-body-input";
const POST_BODY_INPUT_ID_SEL: &str = "#post-body-input";

fn update_text(post: RwSignal<PostDetails>, update: impl FnOnce(&mut String)) {
	println!("we are inside update!");
	log!("we are inside update!");

	post.update(|p| update(&mut p.content));
	let current_len = post.read_untracked().content.len();

	let target = web_sys::window()
		.unwrap()
		.document()
		.unwrap()
		.query_selector(POST_BODY_INPUT_ID_SEL)
		.unwrap()
		.unwrap()
		.dyn_into::<HtmlTextAreaElement>()
		.unwrap();

	target.style("height: auto");
	let scroll_height = target.scroll_height();
	target.style(format!("height: {scroll_height}px"));

	log!("Got before spawn");

	spawn_local(async move {

		log!("Inside spawn, before timeout");

		gloo_timers::future::TimeoutFuture::new((current_len / 100) as u32).await;

		log!("Inside spawn, after timeout");

		let new_len = post.read_untracked().content.len();

		log!("checking equality");

		if new_len == current_len {
			log!("they're equal!");
			// wanna clone this so we don't lock `post` during all of `md_to_html`
			let clone_content = post.read_untracked().content.clone();
			let rendered = md_to_html(&clone_content);
			post.update(|p| p.rendered_content = rendered);
		}
	});
}

#[island]
pub fn edit_post(post: PostDetails) -> impl IntoView {
	let is_new_post = post.id.is_none();
	let post = RwSignal::new(post);
	let (read_asset, write_asset) = signal(AssetUploadState::None);
	let (read_submission, write_submission) = signal(SubmissionState::Preparing);

	let (read_post, write_post) = post.clone().split();
	view!{{ move || match &*read_submission.read() {
        SubmissionState::Resolved(res) => Either::Left(Either::Left(resolved_view(res, is_new_post, write_submission))),
        SubmissionState::Loading => Either::Left(Either::Right("
            <style>
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
            </style>
        ")),
        SubmissionState::Preparing => Either::Right(view!{
            <style>{ BASE_STYLE }</style>
            <style>{ EDIT_STYLE }</style>
            <div id="article-content">
                <h1>"New Post"</h1>
                <input
                    placeholder="title"
                    value=move || read_post.read().title.clone()
                    on:input:target=move |ev| {
                        write_post.update(|p| p.title = ev.target().value());
                        log!("title is now", &read_post.read().title);
                    }
                />
                <br/><br/>
                <div id="edit-and-render">
                    <textarea
                        id={ POST_BODY_INPUT_ID }
                        placeholder="what's goin on? :)"
                        on:input:target=move |ev| update_text(post, |c| *c = ev.target().value())
                        prop:value=move || post.read().content.clone()
                        disabled=move || *read_asset.read() == AssetUploadState::Uploading
                    >
                    {move || read_post.read().content.clone()}
                    </textarea>
                    <span id="rendered" inner_html=move || read_post.read().rendered_content.clone()>
                    </span>
                </div>
                <br/><br/>
                <label
                    for="asset-upload"
                    class="upload-details"
                    on:drop=move |ev| match ev.data_transfer() {
                        Some(dt) => upload_asset(dt.files(), write_asset, post),
                        None => write_asset.set(AssetUploadState::PreflightError(Cow::Borrowed("Event had no DataTransfer")))
                    }
                    on:dragover=move |ev| ev.prevent_default()
                    on:dragenter=move |ev| ev.prevent_default()
                >
                {
                    move || match &*read_asset.read() {
                        AssetUploadState::None => Cow::Borrowed("Drop asset here to upload"),
                        AssetUploadState::PreflightError(err) =>
                            Cow::Owned(format!("Couldn't upload asset: {err}")),
                        AssetUploadState::Uploading => Cow::Borrowed("Uploading asset..."),
                        AssetUploadState::Resolved(res) => match res {
                            Ok((id, true)) => Cow::Owned(format!("Asset uploaded to id {id}")),
                            Ok((id, false)) => Cow::Owned(format!("Asset uploaded to id {id}. processing...")),
                            Err((status, err)) => Cow::Owned(format!("Asset failed to upload with code {status}, error: '{err}'"))
                        }
                    }
                }
                </label>
                <input
                    id="asset-upload"
                    type="file"
                    style="display: none;"
                    on:change:target=move |ev| upload_asset(ev.target().files(), write_asset, post)
                />
                <br/>
                <div id="tags">
                    <h3 id="tag-title">"Tags"</h3>
                    <span>
                        <input id="new-tag-input" on:change:target=move |ev| {
                            let new_tag = ev.target().value();
                            if !new_tag.is_empty() {
                                write_post.update(|p| _ = p.tags.insert(new_tag));
                                ev.target().set_value("");
                            }
                        }/>
                        { move || {
                            read_post.read()
                                .tags
                                .iter()
                                .map(|tag| {
                                    // mm don't like both of the clones but like. Who cares
                                    let tag = tag.to_owned();
                                    let to_remove = tag.clone();
                                    view!{
                                        <span class="tag">
                                            { tag }
                                            <button on:click=move |_| write_post.update(|post| _ = post.tags.remove(&to_remove))>
                                                "x"
                                            </button>
                                        </span>
                                    }
                                })
                                .collect_view()
                        }}
                    </span>
                </div>
                <br/><br/>
                <button id="publish-button" on:click:target=move |_| submit_or_edit_post_outer(read_post, false, write_submission)>
                    "Publish"
                </button>
                { move || {
                    let post = read_post.read();
                    if post.draft || post.id.is_none() {
                        Either::Left(view! {
                            <button id="draft-button" on:click=move |_| submit_or_edit_post_outer(read_post, true, write_submission)>
                                "Save as Draft"
                            </button>
                        })
                    } else {
                        Either::Right(())
                    }
                }}
            </div>
        }),
	}}}
}

fn resolved_view(
	result: &Result<String, String>,
	for_new_post: bool,
	submission: WriteSignal<SubmissionState>
) -> impl IntoView + use<> {
	let res_html = match result.as_ref() {
		Ok(id) => Either::Left(view! {
			<div class="submit-result submit-success">
				<h1 class="submit-title">{
					if for_new_post {
						"Post was created!"
					} else {
						"Post was saved!"
					}
				}</h1>
				<div class="nav-buttons">
					<a href={ format!("/post/{id}") }>{ "View Post" }</a>
					<a href={ format!("/admin/edit_post/{id}") }>{ "Edit Post" }</a>
					<a href="/">"Go Home"</a>
				</div>
			</div>
		}),
		Err(e) => Either::Right(view! {
			<div class="submit-result submit-failure">
				<h1 class="submit-title">{ format!("Submission failed with reason: {e}") }</h1>
				<div class="nav-buttons">
					<button on:click=move |_| submission.set(SubmissionState::Preparing)>
						"Continue Editing"
					</button>
				</div>
			</div>
		})
	};

	view! {
		<style>{ BASE_STYLE }</style>
		<style>{
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
		}</style>
		{ res_html }
	}
}

fn upload_asset(
	files: Option<FileList>,
	asset: WriteSignal<AssetUploadState>,
	post: RwSignal<PostDetails>
) {
	fn prepare_form(files: Option<FileList>) -> Result<FormData, Cow<'static, str>> {
		let files = files.ok_or(Cow::Borrowed("The DataTransfer's file list is None"))?;

		let len = files.length();
		if len != 1 {
			return Err(format!("Only 1 file can be uploaded at a time ({len} were submitted)").into());
		}

		let file = files.item(0)
			.ok_or(Cow::Borrowed("files.item(0) returned None despite verifying one existed in the list"))?;

		let form_data = FormData::new()
			.map_err(|e| Cow::Owned(format!("Couldn't create FormData: {e:?}")))?;

		form_data.append_with_blob_and_filename("file", &file, &file.name())
			.map_err(|e| Cow::Owned(format!("Couldn't append to form: {e:?}")))?;

		Ok(form_data)
	}

	spawn_local(async move {
		let form = match prepare_form(files) {
			Ok(form) => form,
			Err(e) => {
				asset.set(AssetUploadState::PreflightError(e));
				return;
			}
		};

        log!("form: ", &form);

		let form_data = MultipartData::Client(form.into());
		let asset_id = match receive_asset(form_data).await {
			Err(e) => {
				asset.set(AssetUploadState::Resolved(Err((401, format!("{e:?}")))));
				return;
			},
			Ok(u) => u
		};

		let is_image = asset_id.split('.')
			.next_back()
			.is_some_and(|ext| ["jpeg", "jpg", "png", "webp", "heic", "heif"].contains(&ext));
		let exclamation = if is_image { "!" } else { "" };

		update_text(post, |c| writeln!(c, "\n\n{exclamation}[Asset](/api/assets/{asset_id})\n").unwrap());
	});
}

#[server(input = leptos::server_fn::codec::MultipartFormData)]
async fn receive_asset(form: MultipartData) -> Result<String, ServerFnError> {
	let MultipartData::Server(form) = form else {
		return Err(ServerFnError::Deserialization("We got a non-server MultipartData".into()));
	};

	let (session, resp): (tower_sessions::Session, _) =
		backend::ext().await?;

	match crate::images::upload_asset(session, form).await {
		(http::StatusCode::OK, path) => Ok(path),
		(code, reason) => {
			resp.set_status(code);
			Err(ServerFnError::ServerError(reason))
		}
	}
}

fn submit_or_edit_post_outer(
	post: ReadSignal<PostDetails>,
	draft: bool,
	submission: WriteSignal<SubmissionState>
) {
	submission.set(SubmissionState::Loading);

	spawn_local(async move {
		let post = post.read_untracked();
		let resp = submit_or_edit_post(PostReq {
			id: post.id,
			title: post.title.clone(),
			content: post.content.clone(),
			tags: post.tags.clone(),
			draft
		}).await;

		let status = resp.map_err(|e| format!("{e:?}"));
		submission.set(SubmissionState::Resolved(status));
	});
}

#[server(input = Json)]
async fn submit_or_edit_post(req: PostReq) -> Result<String, ServerFnError> {
	use tower_sessions::Session;
	use axum_sqlx_tx::Tx;
	use sqlx::Postgres;

	let ((session, tx), resp): ((Session, Tx<Postgres>), _) = backend::ext().await?;

	let (code, id) = crate::blog_api::submit_post(session, tx, req).await;

	resp.set_status(code);
	Ok(id)
}


const EDIT_STYLE: &str = "
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
";
