use leptos::prelude::*;
use leptos_router::{StaticSegment, ParamSegment, components::{Router, Route, FlatRoutes}};

use super::{main_page::MainPage, rsvp_page::RsvpPage, admin::Admin};

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
