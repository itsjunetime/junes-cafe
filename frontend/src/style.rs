use yew::prelude::*;

const SCHEMES: &[&str] = &[
	// Foresty but bad
	"
	--body-background: #0f1f0f;
	--main-text: #f5e1b9;
	--secondary-text: #d8af7f;
	--main-background: #495635;
	--secondary-background: #5f6f3a;
	--border-color: #464232;
	",
	// Purpleish
	"
	--body-background: #3f3540;
	--main-text: #f1f6ff;
	--secondary-text: #f7ebec;
	--main-background: #1d1e2c;
	--secondary-background: #59656f;
	--border-color: #ac9fbb;
	--title-text: #d1bbe4;
	",
	// dusty
	"
	--body-background: #000000;
	--main-text: #E0AC9D;
	--secondary-text: #A37774;
	--main-background: #484A47;
	--secondary-background: #5C6D70;
	--border-color: #E88873;
	"
];

#[function_component(SharedStyle)]
pub fn shared_style() -> Html {
	html! { <>
		<style>{
			// Ugh I don't like this but oh well
			r#"
			* {
				font-family: Arial;
				color: var(--main-text);
			}
			body {
				background-color: var(--body-background);
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
			pre {
				padding: 10px;
				border-radius: 8px;
				overflow: scroll
			}
			pre > span, code {
				font-family: Courier;
			}
			"#
		}</style>
		<style>{
			format!("* {{ {} }}", SCHEMES[1])
		}</style>
	</> }
}
