// use leptos::prelude::*;

/*pub fn wedding_app(options: LeptosOptions) -> impl IntoView {
	view!{
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<HydrationScripts options=options islands=true/>
			</head>
			<body>
				<MainPage/>
			</body>
		</html>
	}
}*/

use leptos::*;
use leptos_router::{Route, Router, Routes, SsrMode};
use super::main_page::MainPage;

#[component]
pub fn wedding_app() -> impl IntoView {
	view! {
		<Router>
			<Routes>
				<Route path="/" view=MainPage ssr=SsrMode::InOrder/>
			</Routes>
		</Router>
	}
}
