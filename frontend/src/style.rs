use yew::prelude::*;

#[function_component(SharedStyle)]
pub fn shared_style() -> Html {
	html! {
		<style>{
			// Ugh I don't like this but oh well
			r#"
			* {
				font-family: Arial;
				color: var(--main-text);
				--secondary-text: #d8af7f;
				--main-text: #f5e1b9;
				--secondary-background: #5f6f3a;
				--main-background: #495635;
				--border-color: #464232;
			}
			body {
				background-color: #0f1f0f;
			}
			#tag-title {
				color: var(--secondary-text);
			}
			#tag-title ~ br {
				margin-bottom: 10px;
			}
			span.tag {
				margin-right: 8px;
				background-color: var(--secondary-background);
				padding: 4px 6px;
				border-radius: 4px;
				color: var(--main-text)
			}
			input, textarea {
				background-color: var(--secondary-background);
				border: 1px solid var(--border-color);
				border-radius: 4px;
				color: var(--main-text);
			}
			button {
				background-color: var(--main-background);
				border: 1px solid var(--main-background);
				border-radius: 4px;
				padding: 6px 8px;
			}
			"#
		}</style>
	}
}
