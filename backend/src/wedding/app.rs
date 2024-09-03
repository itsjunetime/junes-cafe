use leptos::prelude::*;
use leptos_router::{StaticSegment, ParamSegment, components::{Router, Route, FlatRoutes}};

use super::{main_page::MainPage, server::AxumState, rsvp_page::RsvpPage, admin::Admin};

pub fn wedding_app(state: AxumState) -> impl IntoView {
	let options = state.leptos_opts;

	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<AutoReload options=options.clone()/>
				<HydrationScripts options islands=true />
			</head>
			<body>
				<RouterApp />
			</body>
		</html>
	}
}

#[component]
pub fn router_app() -> impl IntoView {
	view! {
		<Router>
			<main>
				<FlatRoutes fallback=move || "Not found">
					<Route path=(StaticSegment("/rsvp"), ParamSegment("id")) view=RsvpPage />
					<Route path=StaticSegment("/admin") view=Admin />
					<Route path=StaticSegment("") view=MainPage />
				</FlatRoutes>
			</main>
		</Router>
	}
}
