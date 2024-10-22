use std::{io::Cursor, sync::OnceLock};

use syntect::{
	highlighting::{Color, Theme, ThemeSet},
	parsing::SyntaxSet,
	html::{append_highlighted_html_for_styled_line, IncludeBackground},
	easy::HighlightLines,
	util::LinesWithEndings,
};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag, TagEnd};

static FRAPPE_THEME: &[u8] = include_bytes!("../../fonts/catppuccin_syntax/themes/Catppuccin Frappe.tmTheme");

pub fn md_to_html(input: &str) -> String {
	static THEME: OnceLock<Theme> = OnceLock::new();

	let events = pulldown_cmark::Parser::new_ext(input, pulldown_cmark::Options::all());

	let theme = THEME.get_or_init(|| {
		let mut cursor = Cursor::new(FRAPPE_THEME);
		// This only errors on an unknown theme, so we can safely unwrap here
		ThemeSet::load_from_reader(&mut cursor).unwrap()
	});
	let syntax_set = SyntaxSet::load_defaults_newlines();

	// kinda reimplimenting highlight_pulldown::PulldownHighlighter::highlight
	// so that we can do <pre><code> instead of just <pre>
	let mut in_code_block = false;
	let mut syntax = syntax_set.find_syntax_plain_text();
	let mut to_highlight = String::new();

	let events = events.filter_map(|ev| match ev {
		Event::Start(Tag::CodeBlock(kind)) => {
			if let CodeBlockKind::Fenced(lang) = kind {
				if let Some(syn) = syntax_set.find_syntax_by_token(&lang) {
					syntax = syn;
				}
			}
			in_code_block = true;
			None
		},
		Event::End(TagEnd::CodeBlock) => {
			let mut highlighter = HighlightLines::new(syntax, theme);
			let color = theme.settings.background.unwrap_or(Color::BLACK);
			let lang = syntax.name.to_lowercase();
			let mut output = format!(r#"<pre class="language-{lang}"><code class="language-{lang}">"#);

			for line in LinesWithEndings::from(&to_highlight) {
				// if we fail to highlight, it's kinda whatever
				if let Ok(regions) = highlighter.highlight_line(line, &syntax_set) {
					_ = append_highlighted_html_for_styled_line(
						&regions[..],
						IncludeBackground::IfDifferent(color),
						&mut output
					)
				}
			}

			output.push_str("</code></pre>\n");

			to_highlight.clear();
			in_code_block = false;
			Some(Event::Html(CowStr::from(output)))
		},
		Event::Text(t) if in_code_block => {
			to_highlight.push_str(&t);
			None
		},
		e => Some(e)
	});

	let mut html = String::new();

	// So it would be smart to sanitize the html to make sure that XSS and stuff like that isn't
	// supported but it's my website and I think it's fun to have the option of doing fun little
	// stuff with javascript if I would so like, and this input is already trusted (since only
	// logged-in users can access this API and I am the only user) so I don't see the need to
	// sanitize very strongly
	pulldown_cmark::html::push_html(&mut html, events);

	html
}
