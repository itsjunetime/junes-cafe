use yew::prelude::*;
use super::{
	style::SharedStyle,
	GetPostErr
};
use chrono::NaiveDateTime;

#[derive(Properties, PartialEq, Eq)]
pub struct PostProps {
	pub id: u32 
}

#[function_component(ViewPost)]
pub fn view_post(props: &PostProps) -> Html {
	let post = use_state(|| None);

	{
		let id = props.id.to_owned();
		let state = post.clone();
		use_effect(move || {
			if state.is_none() {
				super::get_post(id, state);
			}

			|| ()
		});
	}

	let post_html = match post.as_ref() {
		None => html! { <p>{ "Retrieving post..." }</p> },
		Some(Err(GetPostErr::NotFound)) => html! { <h1>{ "Not Found" }</h1> },
		Some(Err(GetPostErr::Other(err))) => html! { <><h1>{ "Error" }</h1><p>{ err }</p></> },
		Some(Ok(post)) => html! {
			<div id="post-content">
				<span id="post-header">
					<h2 id="post-title">{ &post.title }</h2>
					<span>{ "At " }
						<strong>{
							NaiveDateTime::from_timestamp_opt(post.created_at as i64, 0)
								.map(|dt| dt.format("%H:%M on %b %-d, %Y").to_string())
								.unwrap_or_else(|| "an unknown time".into())
						}</strong>
						{ " by " }
						<strong>{ post.display_user() }</strong>
						{ "; " }{ &post.reading_time }{ " minute read" }
					</span>
				</span>
				<br /><br />
				<div id="post-text">
				{ Html::from_html_unchecked(post.html.clone().into()) }
				</div>
				{
					if !post.tags.0.is_empty() {
						html! {
							<>
								<br /><br />
								<div id="tags">
									<span id="tag-title">{ "Tags" }</span>
									<br />
									{
										post.tags.0.iter().map(|tag|
											html! { <span class="tag">{ tag }</span> }
										).collect::<Html>()
									}
								</div>
							</>
						}
					} else {
						html! { }
					}
				}
			</div>
		}
	};

	html! {
		<>
			<SharedStyle />
			<style>
			{
				"
				#post-content {
					max-width: 790px;
					margin: 10px auto;
				}
				#post-header * {
					color: var(--secondary-text);
				}
				#post-title {
					color: var(--title-text);
				}
				#post-text {
					color: var(--main-text);
					padding: 12px 12px;
					border-radius: 4px;
				}
				#post-text img {
					max-width: 100%;
				}
				"
			}
			</style>
			{ post_html }
		</>
	}
}
