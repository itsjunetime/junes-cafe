pub mod wedding;
#[cfg(not(target_family = "wasm"))]
pub mod auth;

// necessary for wasm_bindgen to find islands to hydrate
#[expect(unused_imports)]
use wedding::*;

#[cfg(not(target_family = "wasm"))]
pub mod state {
	use axum::extract::{FromRef, FromRequestParts};
	use axum_sqlx_tx::State;
	use sqlx::Postgres;
	use http::request::Parts;
	use std::{future::Future, pin::Pin};
	use leptos::prelude::*;

	pub async fn ext<T>() -> Result<(T, leptos_axum::ResponseOptions), ServerFnError>
	where
		T: FromRequestParts<AxumState>,
		<T as FromRequestParts<AxumState>>::Rejection: std::fmt::Debug
	{
		let state: AxumState = expect_context();
		leptos_axum::extract_with_state(&state).await
			.map(|t| (t, expect_context()))
	}

	#[derive(Clone)]
	pub struct AxumState {
		pub tx_state: State<Postgres>,
		pub leptos_opts: LeptosOptions
	}

	impl FromRef<AxumState> for State<Postgres> {
		fn from_ref(input: &AxumState) -> Self {
			input.tx_state.clone()
		}
	}

	impl FromRef<AxumState> for LeptosOptions {
		fn from_ref(input: &AxumState) -> Self {
			input.leptos_opts.clone()
		}
	}

	impl FromRequestParts<AxumState> for State<Postgres> {
		type Rejection = std::convert::Infallible;

		fn from_request_parts<
			'life0,
			'life1,
			'async_trait
		>(
			_parts: &'life0 mut Parts,
			state: &'life1 AxumState
		) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
		where
			'life0: 'async_trait,
			'life1: 'async_trait,
			Self: 'async_trait
		{
			Box::pin(async move { Ok(state.tx_state.clone()) })
		}
	}

	pub fn leptos_app<V>(
		state: AxumState,
		#[expect(non_snake_case)] // leptos won't actually render it unless it's non-snake-case
		Router: impl Fn() -> V
	) -> impl IntoView
	where
		V: IntoView
	{
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
					<Router />
				</body>
			</html>
		}
	}
}

#[cfg(not(target_family = "wasm"))]
pub use state::*;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
	#[cfg(target_family = "wasm")]
	console_error_panic_hook::set_once();

	#[cfg(feature = "hydrate")]
	leptos::mount::hydrate_islands();
}
