#[cfg(not(target_family = "wasm"))]
pub mod auth;

#[cfg(not(target_family = "wasm"))]
pub mod state {
	use axum::extract::{FromRef, FromRequestParts};
	use axum_sqlx_tx::State;
	use sqlx::Postgres;
	use http::request::Parts;
	use tower_cache::invalidator::Invalidator;
	use leptos::prelude::*;

	#[derive(Clone)]
	pub struct AxumState {
		pub tx_state: State<Postgres>,
		pub leptos_opts: LeptosOptions,
		pub invalidator: Invalidator
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

	impl FromRef<AxumState> for Invalidator {
		fn from_ref(input: &AxumState) -> Self {
			input.invalidator.clone()
		}
	}

	impl FromRequestParts<AxumState> for State<Postgres> {
		type Rejection = std::convert::Infallible;

		async fn from_request_parts(
			_parts: &mut Parts,
			state: &AxumState
		) -> Result<Self, Self::Rejection> {
			Ok(state.tx_state.clone())
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
