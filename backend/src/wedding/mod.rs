pub mod main_page;
pub mod server;

#[cfg(not(target_family = "wasm"))]
pub mod app;
#[cfg(not(target_family = "wasm"))]
pub mod faq;

#[cfg(not(target_family = "wasm"))]
use ::{
    const_format::concatcp,
    sqlx::{query, PgPool},
};
#[cfg(not(target_family = "wasm"))]
use server::{GUESTS_TABLE, RECIPS_TABLE};

pub const SHARED_STYLE: &str = r#"
@import url('https://fonts.googleapis.com/css2?family=Euphoria+Script&display=swap');
* {
	--gold: #8a944d;
	--white: #eff1f3;
	--olive-green: #8d7c3d;
	--soft-brown: #7c59ec;
	--tan: #9b8461;
	--beige: #cbb9b7;
	--dark-brown: #40332a;
	color: var(--dark-brown);
	font-family: "Euphoria Script", Arial;
}
"#;

#[cfg(not(target_family = "wasm"))]
pub async fn create_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
	query(concatcp!("CREATE TABLE IF NOT EXISTS ", GUESTS_TABLE, "(
		id uuid PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
		name text NOT NULL,
		party_size INT NOT NULL,
		full_address text,
		email text,
		extra_notes text
	);")).execute(pool)
		.await?;

	query(concatcp!("CREATE TABLE IF NOT EXISTS ", RECIPS_TABLE, "(
		id serial PRIMARY KEY,
		name text NOT NULL,
		address text,
		email text
	);")).execute(pool)
		.await?;

	Ok(())
}
