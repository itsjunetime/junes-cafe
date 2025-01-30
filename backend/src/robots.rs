use crate::blog_api::get_post_list;
use axum_sqlx_tx::Tx;
use sqlx::Postgres;
use axum::{http::StatusCode, response::IntoResponse};
use sitewriter::{UrlEntry, ChangeFreq};
use chrono::DateTime;
use rss::{Item, Category, Source, Channel};

use std::{borrow::Cow, fmt::Display};

#[cfg_attr(not(debug_assertions), expect(dead_code))]
pub(crate) struct ErrorOnDebug<E> {
	error: E
}

impl<E> IntoResponse for ErrorOnDebug<E> where E: Display {
	fn into_response(self) -> axum::response::Response {
		#[cfg(debug_assertions)]
		let tup = (StatusCode::INTERNAL_SERVER_ERROR, Cow::Owned(format!("Internal Error: {}", self.error)));

		#[cfg(not(debug_assertions))]
		let tup = (StatusCode::INTERNAL_SERVER_ERROR, Cow::Borrowed("We ran into an issue. Please try again later."));

		<(_, Cow<'static, str>) as IntoResponse>::into_response(tup)
	}
}

impl<E> From<E> for ErrorOnDebug<E> {
	fn from(error: E) -> Self {
		Self { error }
	}
}

pub async fn get_sitemap_xml(mut tx: Tx<Postgres>) -> Result<String, ErrorOnDebug<sqlx::Error>> {
	let posts = get_post_list(None, &mut tx, i32::MAX as u32, 0).await?;

	let urls = posts.into_iter()
		.map(|post| UrlEntry {
			loc: format!("https://itsjuneti.me/post/{}", post.id).parse().unwrap(),
			lastmod: DateTime::from_timestamp(post.created_at as i64, 0),
			changefreq: Some(ChangeFreq::Never),
			priority: None
		})
		.collect::<Vec<_>>();

	Ok(sitewriter::generate_str(&urls))
}

pub async fn get_rss_xml(mut tx: Tx<Postgres>) -> Result<String, ErrorOnDebug<sqlx::Error>> {
	let posts = get_post_list(None, &mut tx, i32::MAX as u32, 0).await?;

	let last_update = posts.iter()
		.map(|p| p.created_at)
		.max();

	let items = posts.into_iter()
		.map(|post| Item {
			title: Some(post.title),
			link: Some(format!("https://itsjuneti.me/post/{}", post.id)),
			author: Some("junewelker@gmail.com".into()),
			categories: post.tags.0
				.into_iter()
				.map(|t| Category {
					name: t,
					domain: None
				})
				.collect(),
			pub_date: DateTime::from_timestamp(post.created_at as i64, 0)
				.map(|dt| dt.to_rfc2822()),
			source: Some(Source {
				url: "https://itsjuneti.me/index.xml".into(),
				title: None
			}),
			content: Some(post.html),
			..Item::default()
		})
		.collect::<Vec<_>>();

	let channel = Channel {
		title: "itsjuneti.me".into(),
		link: "https://itsjuneti.me".into(),
		description: "A blog about various tech topics but mainly rust".into(),
		language: Some("en_US".into()),
		managing_editor: Some("junewelker@gmail.com".into()),
		webmaster: Some("junewelker@gmail.com".into()),
		last_build_date: last_update
			.and_then(|ts| DateTime::from_timestamp(ts as i64, 0))
			.map(|dt| dt.to_rfc2822()),
		categories: vec![
			Category {
				name: "Technology".into(),
				domain: None
			},
			Category {
				name: "Rustlang".into(),
				domain: None
			}
		],
		generator: Some("https://crates.io/crates/rss".into()),
		ttl: Some("1440".into()),
		items,
		..Channel::default()
	};

	Ok(channel.to_string())
}

pub async fn get_robots_txt() -> &'static str {
	tower_no_ai::bot_blocking_robots_txt()
}
