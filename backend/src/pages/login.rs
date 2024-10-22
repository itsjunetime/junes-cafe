use horrorshow::{helper::doctype, html, Raw};
use axum::{extract::Query, response::Html};
use backend::auth::LoginQuery;

pub async fn login_html(Query(query): Query<LoginQuery>) -> Html<String> {
	Html(html! {
		: doctype::HTML;
		html(lang = "en") {
			head {
				title : "Login";
				style : Raw(shared_data::BASE_STYLE);
				style : Raw(r"
					button:hover {
						background-color: #00000000;
					}
					#login-form {
						max-width: max-content;
						margin: auto;
					}
					input {
						font-size: 20px;
					}
					.login-status {
						color: red;
					}
				");
				meta(name = "viewport", content = "width=device-width, initial-scale=1");
			}
			body {
				form(action = "/api/login", method = "POST", id = "login-form") {
					h1 : "Login";
					br;
					input(placeholder = "username", type = "text", name = "username", autocomplete = "username");
					br; br;
					input(placeholder = "password", type = "password", name = "password", autocomplete = "current-password");
					br;
					@ if let Some(ref err_msg) = query.err_msg {
						span(class = "login-status") : err_msg;
					}
					@ if let Some(ref redir) = query.redir_to {
						input(type = "text", name = "redir_to", value = redir, style = "display: none;");
					}
					br; br;
					input(type = "submit", value = "Login")
				}
			}
		}
	}.to_string())
}
