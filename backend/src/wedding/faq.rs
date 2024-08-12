// Items for the FAQ:
// - Do you have a registry?
//   We're still working on setting that up! This site should update once we've set it up, and if
//   you register for an announcement before we've set up the registry, we'll send you an email
//   with a link to it.
// - How can I get an invite?
//   We'll be sending out invites soon, so stay posted!
// - Where will you be having the wedding?
//   We'll let you know on the invite :)
// - When will it be?
//   December 14, 2024, in the evening.
// - This site is so cool! Can I view its source code?
//   Yes! Here's the link to the repo:

use axum::response::Html;
use const_format::concatcp;

pub async fn wedding_faq() -> Html<&'static str> {
	Html(concatcp!(r#"
		<body>
			<style>
"#,
super::SHARED_STYLE,
r#"
* {
	font-size: 24px;
}
body {
	background-color: var(--beige);
}
#main-content {
	max-width: 900px;
	margin: 0 auto;
}
h1 {
	font-size: 64px;
}
.answer {
	margin-left: 10px;
}
.question {
	border-bottom: 2px solid var(--white);
	width: max-content;
}
			</style>
			<div id="main-content">
				<h1>Maggie &amp; June's Wedding FAQ</h1>

				<div class="q-and-a">
					<h3 class="question">Do you have a registry?</h3>
					<p class="answer">Yes! You can find it <a href="https://www.zola.com/registry/maggieandjune">here, on Zola.</a></p>
				</div>
				<div class="q-and-a">
					<h3 class="question">How can I get an invite?</h3>
					<p class="answer">We'll be sending out invites soon, so stay posted!</p>
				</div>
				<div class="q-and-a">
					<h3 class="question">Where will you be having the wedding?</h3>
					<p class="answer">We'll let you know on the invite :)</p>
				</div>
				<div class="q-and-a">
					<h3 class="question">When will it be?</h3>
					<p class="answer">December 14, 2024, in the evening.</p>
				</div>
				<div class="q-and-a">
					<h3 class="question">This site is so cool! Can I view its source code?</h3>
					<p class="answer">Absolutely! <a href="https://github.com/itsjunetime/junes-cafe">Here's the link to the repo!</a></p>
				</div>
			</div>
		</body>
	"#))
}
