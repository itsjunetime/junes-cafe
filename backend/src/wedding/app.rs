use leptos::prelude::*;
use leptos_router::{StaticSegment, components::{Router, Route, FlatRoutes}};

use super::{main_page::MainPage, server::AxumState};

pub fn wedding_app(state: AxumState) -> impl IntoView {
	let options = state.leptos_opts;

	view!{
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<HydrationScripts options=options islands=true/>
			</head>
			<body>
				<RouterApp/>
			</body>
		</html>
	}
}

#[component]
fn router_app() -> impl IntoView {
	view! {
		<Router>
			<main>
				<FlatRoutes fallback=move || "Not found">
					<Route path=StaticSegment("") view=MainPage />
				</FlatRoutes>
			</main>
		</Router>
	}
}

/*use leptos::*;
use leptos_router::{Route, Router, Routes, SsrMode};

#[component]
pub fn wedding_app() -> impl IntoView {
	view! {
		<Router>
			<Routes>
				<Route path="/" view=MainPage ssr=SsrMode::InOrder/>
			</Routes>
		</Router>
	}
}*/
