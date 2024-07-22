mod wedding;
#[cfg(not(target_family = "wasm"))]
mod auth;

#[allow(unused_imports)]
use wedding::*;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
	#[cfg(target_family = "wasm")]
	console_error_panic_hook::set_once();

	// for 0.7:
	leptos::mount::hydrate_islands();
}
