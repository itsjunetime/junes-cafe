use leptos::prelude::*;

#[cfg(not(target_family = "wasm"))]
pub use server_side::*;

#[cfg(not(target_family = "wasm"))]
macro_rules! ext{
	() => {{
		let state: server_side::AxumState = expect_context();
		extract_with_state(&state).await
			.map(|t| (t, expect_context()))
	}};
}

// ideally this would take an AnnouncementRecipient as an argument but I can't figure out how to
// make that work
#[server(prefix = "/wedding_api")]
pub async fn add_announcement_req(
	name: String,
	address: String,
	email: String
) -> Result<(), ServerFnError> {
	use axum_sqlx_tx::Tx;
	use sqlx::{Postgres, query};
	use axum::http::StatusCode;
	use leptos_axum::{ResponseOptions, extract_with_state};
	use const_format::concatcp;

	let (mut tx, response): (Tx<Postgres>, ResponseOptions) = ext!()?;

	if name.is_empty() || address.is_empty() {
		response.set_status(StatusCode::BAD_REQUEST);
		return Err(ServerFnError::ServerError("Both name and address must be non-empty".into()));
	}

	query(concatcp!(
		"INSERT INTO ", RECIPS_TABLE,
		" (name, address, email) VALUES ($1, $2, $3)
		ON CONFLICT DO NOTHING"
	))
		.bind(name)
		.bind(address)
		.bind(email)
		.execute(&mut tx)
		.await
		.map_err(|e| {
			response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
			ServerFnError::ServerError(format!("Couldn't insert data: {e}"))
		})
		.map(|_| ())
}

#[cfg(not(target_family = "wasm"))]
mod server_side {
	use std::fmt::Debug;
	use axum::{extract::{FromRequestParts, FromRef}, http::StatusCode};
	use axum_sqlx_tx::{State, Tx};
	use const_format::concatcp;
	use leptos::prelude::*;
	// use leptos::*;
	use leptos_axum::{extract_with_state, ResponseOptions};
	use serde::{Serialize, Deserialize};
	use sqlx::{query, query_as, FromRow, Postgres, Row};
	use tower_sessions::Session;
	use uuid::Uuid;

	use crate::check_auth;

	pub const GUESTS_TABLE: &str = "wedding_guests";
	pub const RECIPS_TABLE: &str = "announcement_recipients";

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

	#[derive(FromRow, Serialize, Deserialize, Clone, Debug)]
	pub struct Guest {
		id: Uuid,
		name: String,
		#[sqlx(try_from = "i32")]
		party_size: PartySize,
		full_address: Option<String>,
		email: Option<String>,
		extra_notes: String,
	}

	impl Guest {
		pub fn has_rsvpd(&self) -> bool {
			self.email.is_some()
		}
	}

	#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
	#[cfg_attr(test, derive(PartialEq))]
	enum PartySize {
		// this is a party of multiple people
		Group(u8),
		// a single person who is not allowed a plus one
		NoPlusOne,
		// a single person who is allowed a plus one but has not specified whether they will be
		// bringing one
		AllowedPlusOne,
		// a single person who is allowed a plus one but will not be bringing one
		NotBringing,
		// a single person who is allowed a plus one and will be bringing one
		Bringing
	}

	impl PartySize {
		pub fn total_size(&self) -> u8 {
			match self {
				Self::Group(size) => *size,
				// if they haven't specified, just assume it's a no. for now. i guess.
				Self::NoPlusOne | Self::NotBringing | Self::AllowedPlusOne => 1,
				Self::Bringing => 2,
			}
		}
	}

	#[cfg_attr(test, derive(Debug, PartialEq))]
	struct UnknownTag(u8);

	// so this is kinda janky but it allows us to store this into the database
	impl TryFrom<i32> for PartySize {
		type Error = UnknownTag;
		fn try_from(value: i32) -> Result<Self, Self::Error> {
			// this is just to ensure we don't run into any snags with the necessary bit shifting and
			// the signed bit and such. I'm pretty certain it's basically invisible
			let le_bytes = value.to_le_bytes();
			let tag = le_bytes[0];

			match tag {
				0 => Ok(Self::Group(le_bytes[3])),
				1 => Ok(Self::NoPlusOne),
				2 => Ok(Self::AllowedPlusOne),
				3 => Ok(Self::NotBringing),
				4 => Ok(Self::Bringing),
				_ => Err(UnknownTag(tag))
			}
		}
	}

	impl From<PartySize> for i32 {
		fn from(value: PartySize) -> Self {
			match value {
				PartySize::Group(num) => i32::from_le_bytes([0, 0, 0, num]),
				PartySize::NoPlusOne => 1,
				PartySize::AllowedPlusOne => 2,
				PartySize::NotBringing => 3,
				PartySize::Bringing => 4
				// WHENEVER YOU UPDATE THIS, MAKE SURE TO UPDATE THE TryFrom<i32> AS WELL TO MATCH
			}
		}
	}

	#[cfg(test)]
	#[test]
	fn all_party_sizes_can_serde() {
		fn check(size: PartySize) {
			assert_eq!(PartySize::try_from(i32::from(size)), Ok(size));
		}

		for i in u8::MIN..=u8::MAX {
			check(PartySize::Group(i));
		}
		check(PartySize::NoPlusOne);
		check(PartySize::AllowedPlusOne);
		check(PartySize::NotBringing);
		check(PartySize::Bringing);
	}

	// we don't need a key for this struct 'cause we never need to select individuals from it. We're
	// just gonna look at the whole list and check them off one by one as we send out invitations
	#[derive(Serialize, Deserialize, FromRow, Clone, Debug)]
	pub struct AnnouncementRecipient {
		name: String,
		address: String,
		email: String
	}

	enum RetrievalErr {
		NotAllowed,
		Sqlx(sqlx::Error)
	}

	async fn ext<T>() -> Result<(T, ResponseOptions), ServerFnError>
	where
	T: FromRequestParts<AxumState>,
	<T as FromRequestParts<AxumState>>::Rejection: Debug
	{
		let state: AxumState = expect_context::<AxumState>();
		extract_with_state(&state).await
			.map(|t| (t, expect_context()))
	}

	async fn is_june_auth(session: Session, resp: &ResponseOptions) -> Result<(), ServerFnError> {
		match check_auth!(session, noret) {
			Some(username) if username == "june" => Ok(()),
			_ => {
				resp.set_status(StatusCode::UNAUTHORIZED);
				Err(ServerFnError::ServerError("You're not allowed to access this".into()))
			}
		}
	}

	async fn guest_with_id(
		mut tx: Tx<Postgres>,
		id: Uuid
	) -> Result<Guest, RetrievalErr> {
		query_as(concatcp!("SELECT * FROM ", GUESTS_TABLE, " WHERE id = $1"))
			.bind(id)
			.fetch_one(&mut tx)
			.await
			.map_err(RetrievalErr::Sqlx)
	}

	#[derive(Deserialize, Serialize)]
	pub enum Relation {
		AnnouncementOnly(AnnouncementRecipient),
		Invitee(Guest),
	}

	#[server(prefix = "/wedding_api")]
	async fn all_announcement_recipients() -> Result<Vec<Relation>, ServerFnError> {
		let ((mut tx, session), response): ((Tx<Postgres>, _), _) = ext().await?;

		is_june_auth(session, &response).await?;

		let guests = query_as(concatcp!("SELECT * FROM ", GUESTS_TABLE))
			.fetch_all(&mut tx)
			.await?
			.into_iter()
			.map(Relation::Invitee);

		let recips = query_as(concatcp!("SELECT * FROM ", RECIPS_TABLE))
			.fetch_all(&mut tx)
			.await?
			.into_iter()
			.map(Relation::AnnouncementOnly);

		Ok(guests.chain(recips).collect())
	}

	#[server(prefix = "/wedding_api")]
	async fn update_rsvp(guest: Guest) -> Result<(), ServerFnError> {
		let (mut tx, response): (Tx<Postgres>, _) = ext().await?;

		query(concatcp!(
			"UPDATE ", GUESTS_TABLE,
			"SET name = $1, party_size = $2, full_address = $3, email = $4, extra_notes = $5
			WHERE id = $6"
		))
			.bind(guest.name)
			.bind(i32::from(guest.party_size))
			.bind(guest.full_address)
			.bind(guest.email)
			.bind(guest.extra_notes)
			.bind(guest.id)
			.execute(&mut tx)
			.await
			.map_err(|e| {
				response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
				ServerFnError::ServerError(format!("Couldn't update rsvp: {e}"))
			})
			.map(|_| ())
	}

	#[derive(Serialize, Deserialize, Clone, Debug)]
	pub struct NewGuest {
		name: String,
		party_size: PartySize
	}

	#[server(prefix = "/wedding_api")]
	pub async fn add_guest(guest: NewGuest) -> Result<String, ServerFnError> {
		let ((session, mut tx), response): ((_, Tx<Postgres>), _) = ext().await?;

		is_june_auth(session, &response).await?;

		let name = guest.name;

		query(concatcp!(
			"INSERT INTO ", GUESTS_TABLE,
			"(name, party_size) VALUES ($1, $2)
			RETURNING id"
		))
			.bind(&name)
			.bind(i32::from(guest.party_size))
			.fetch_one(&mut tx)
			.await
			.map_err(|e| {
				response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
				ServerFnError::ServerError(format!("Failed to add new guest: {e}"))
			})
			.map(|r| r.try_get::<Uuid, _>("id")
					.map_or_else(
						|e| {
							response.set_status(StatusCode::CREATED);
							format!("Guest {name} added, but returned no id: {e}")
						},
						|i| i.to_string()
					)
			)
	}
}

// pages to contain:
// - FAQ
// - Little basic landing page (maybe people can put in their address+name here as well if they
//   want an announcement)
//   - this may actually benefit from wasm - we have a form that reloads some small bit of the page
//     when they hit submit
// - Place where specific guests can put in details about themselves attending the ceremony or
//   reception
// - Page after inputting details that says like 'email us at <email> if you want to change
//   anything. the last date to change details is dec 1. whatever.
