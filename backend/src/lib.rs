pub mod wedding;
#[cfg(not(target_family = "wasm"))]
pub mod auth;

// necessary for wasm_bindgen to find islands to hydrate
#[expect(unused_imports)]
use wedding::*;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
	#[cfg(target_family = "wasm")]
	console_error_panic_hook::set_once();

	#[cfg(feature = "hydrate")]
	leptos::mount::hydrate_islands();
}
