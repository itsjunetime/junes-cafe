use yew::prelude::*;

#[function_component(SharedStyle)]
pub fn shared_style() -> Html {
	html! { <style>{ shared_data::BASE_STYLE }</style> }
}
