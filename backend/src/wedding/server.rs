use leptos::prelude::*;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[cfg(not(target_family = "wasm"))]
pub use server_side::*;

#[cfg(not(target_family = "wasm"))]
use ::{
	axum::http::StatusCode,
	axum_sqlx_tx::Tx,
	sqlx::{Postgres, query_as, query},
	const_format::concatcp
};

#[cfg(not(target_family = "wasm"))]
async fn ext<T>() -> Result<(T, leptos_axum::ResponseOptions), ServerFnError>
where
	T: axum::extract::FromRequestParts<AxumState>,
	<T as axum::extract::FromRequestParts<AxumState>>::Rejection: std::fmt::Debug
{
	let state: AxumState = expect_context();
	leptos_axum::extract_with_state(&state).await
		.map(|t| (t, expect_context()))
}

// ideally this would take an AnnouncementRecipient as an argument but I can't figure out how to
// make that work
#[server(prefix = "/wedding_api")]
pub async fn add_announcement_req(
	name: String,
	address: String,
	email: String
) -> Result<(), ServerFnError> {
	let (mut tx, response): (Tx<Postgres>, _) = ext().await?;

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

impl Guest {
	pub fn has_rsvpd(&self) -> bool {
		self.email.is_some()
	}
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum PartySize {
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

	pub const fn to_int(self) -> i32 {
		match self {
			PartySize::Group(num) => i32::from_le_bytes([0, 0, 0, num]),
			PartySize::NoPlusOne => 1,
			PartySize::AllowedPlusOne => 2,
			PartySize::NotBringing => 3,
			PartySize::Bringing => 4
			// WHENEVER YOU UPDATE THIS, MAKE SURE TO UPDATE THE TryFrom<i32> AS WELL TO MATCH
		}
	}
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct UnknownTag(u8);

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
		value.to_int()
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(not(target_family = "wasm"), derive(sqlx::FromRow))]
pub struct Guest {
	pub id: Uuid,
	pub name: String,
	#[cfg_attr(not(target_family = "wasm"), sqlx(try_from = "i32"))]
	pub party_size: PartySize,
	pub full_address: Option<String>,
	pub email: Option<String>,
	pub extra_notes: String,
}

#[server(prefix = "/wedding_api")]
pub async fn guest_with_id(id: Uuid) -> Result<Option<Guest>, ServerFnError> {
	let (mut tx, response): (Tx<Postgres>, _) = ext().await?;

	let query_resp = query_as(concatcp!("SELECT * FROM ", GUESTS_TABLE, " WHERE id = $1"))
		.bind(id)
		.fetch_one(&mut tx)
		.await;

	match query_resp {
		Ok(g) => Ok(Some(g)),
		Err(sqlx::Error::RowNotFound) => Ok(None),
		Err(e) => {
			response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
			Err(ServerFnError::ServerError(format!("Couldn't query database: {e}")))
		}
	}
}

// unfortunately, this API has to be designed to work with html forms, so that's why we got this
// weirdness in types here.
#[server(prefix = "/wedding_api")]
async fn update_rsvp(
	accepted_plus_one: Option<bool>,
	group_size: Option<u8>,
	full_address: String,
	email: String,
	extra_notes: String,
	id: Uuid,
) -> Result<(), ServerFnError> {
	static GROUP_SIZE_COND: &str = concatcp!(
		"party_size BETWEEN ", PartySize::Group(0).to_int(), " AND ", PartySize::Group(u8::MAX).to_int()
	);
	static PLUS_ONE_COND: &str = concatcp!(
		"(party_size is ", PartySize::AllowedPlusOne.to_int(),
		" OR ", PartySize::NotBringing.to_int(),
		" OR ", PartySize::Bringing.to_int(), ")"
	);
	static ALONE_COND: &str = concatcp!("party_size IS ", PartySize::NoPlusOne.to_int());

	let (mut tx, response): (Tx<Postgres>, _) = ext().await?;

	let (party_size, extra_cond) = match (accepted_plus_one, group_size) {
		// arbitrarily make group_size override accepted_plus_one. If they submit both, act as if
		// they only submitted group_size
		(_, Some(size)) => (PartySize::Group(size), GROUP_SIZE_COND),
		(Some(accepted), None) => (
			if accepted { PartySize::Bringing } else { PartySize::NotBringing },
			PLUS_ONE_COND
		),
		(None, None) => (PartySize::NoPlusOne, ALONE_COND)
	};

	query(&format!(
		"UPDATE {GUESTS_TABLE} SET party_size = $1, full_address = $2, email = $3, extra_notes = $4 WHERE id = $5 AND {extra_cond}"
	))
		.bind(i32::from(party_size))
		.bind(full_address)
		.bind(email)
		.bind(extra_notes)
		.bind(id)
		.execute(&mut tx)
		.await
		.map_err(|e| match e {
			sqlx::Error::RowNotFound => {
				response.set_status(StatusCode::BAD_REQUEST);
				ServerFnError::ServerError("No guest was found with the provided data (did you mess with the form?)".into())
			},
			_ => {
				response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
				ServerFnError::ServerError(format!("Couldn't update rsvp: {e}"))
			}
		})
		.map(|_| ())
}

#[cfg(not(target_family = "wasm"))]
mod server_side {
	use axum::{extract::FromRef, http::StatusCode};
	use axum_sqlx_tx::{State, Tx};
	use const_format::concatcp;
	use leptos::prelude::*;
	use leptos_axum::ResponseOptions;
	use serde::{Serialize, Deserialize};
	use sqlx::{query, query_as, FromRow, Postgres, Row};
	use tower_sessions::Session;
	use uuid::Uuid;

	use crate::check_auth;
	use super::{Guest, PartySize, ext};

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


	// we don't need a key for this struct 'cause we never need to select individuals from it. We're
	// just gonna look at the whole list and check them off one by one as we send out invitations
	#[derive(Serialize, Deserialize, FromRow, Clone, Debug)]
	pub struct AnnouncementRecipient {
		name: String,
		address: String,
		email: String
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
