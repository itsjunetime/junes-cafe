use crate::print_and_ret;
use std::{sync::Arc, collections::BTreeMap};
use tokio::sync::RwLock;
use axum_sqlx_tx::Tx;
use shared_data::sqlx::{self, Postgres};
use axum::http::StatusCode;
use sitewriter::{UrlEntry, ChangeFreq};
use chrono::DateTime;
use once_cell::sync::Lazy;
use rss::{Item, Category, Source, Channel};

static SITEMAP_XML: Lazy<Arc<RwLock<String>>> = Lazy::new(Arc::default);
static RSS_XML: Lazy<Arc<RwLock<String>>> = Lazy::new(Arc::default);

pub async fn update_sitemap_xml(tx: &mut Tx<Postgres>) -> Result<(), sqlx::error::Error> {
	let urls = crate::get_post_list(None, tx, i32::MAX as u32, 0).await?
		.into_iter()
		.map(|post| UrlEntry {
			loc: format!("https://itsjuneti.me/post/{}", post.id).parse().unwrap(),
			lastmod: DateTime::from_timestamp(post.created_at as i64, 0),
			changefreq: Some(ChangeFreq::Never),
			priority: None
		})
		.collect::<Vec<_>>();

	let xml = sitewriter::generate_str(&urls);

	let mut sitemap = SITEMAP_XML.write().await;
	*sitemap = xml;

	Ok(())
}

pub async fn get_sitemap_xml(mut tx: Tx<Postgres>) -> (StatusCode, String) {
	if SITEMAP_XML.read().await.is_empty() &&
		update_sitemap_xml(&mut tx).await.is_err() {
			print_and_ret!("Couldn't update sitemap.xml")
		}

	(StatusCode::OK, SITEMAP_XML.read().await.clone())
}

pub async fn update_rss_xml(tx: &mut Tx<Postgres>) -> Result<(), Box<dyn std::error::Error>> {
	let posts = crate::get_post_list(None, tx, i32::MAX as u32, 0).await?;

	let last_update = posts.iter()
		.map(|p| p.created_at)
		.max();

	let items = posts.into_iter()
		.map(|post| Item {
			title: Some(post.title),
			link: Some(format!("https://itsjuneti.me/post/{}", post.id)),
			description: None,
			author: Some("junewelker@gmail.com".into()),
			categories: post.tags.0
				.into_iter()
				.map(|t| Category {
					name: t,
					domain: None
				})
				.collect(),
			comments: None,
			enclosure: None,
			guid: None,
			pub_date: DateTime::from_timestamp(post.created_at as i64, 0)
				.map(|dt| dt.to_rfc2822()),
			source: Some(Source {
				url: "https://itsjuneti.me/index.xml".into(),
				title: None
			}),
			content: Some(post.html),
			extensions: BTreeMap::new(),
			itunes_ext: None,
			dublin_core_ext: None
		})
		.collect::<Vec<_>>();

	let channel = Channel {
		title: "itsjuneti.me".into(),
		link: "https://itsjuneti.me".into(),
		description: "A blog about various tech topics but mainly rust".into(),
		language: Some("en_US".into()),
		copyright: None,
		managing_editor: Some("junewelker@gmail.com".into()),
		webmaster: Some("junewelker@gmail.com".into()),
		pub_date: None,
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
		docs: None,
		cloud: None,
		rating: None,
		ttl: Some("1440".into()),
		image: None,
		text_input: None,
		skip_hours: vec![],
		skip_days: vec![],
		items,
		extensions: BTreeMap::new(),
		itunes_ext: None,
		dublin_core_ext: None,
		syndication_ext: None,
		namespaces: BTreeMap::new()
	};

	let mut rss_xml = RSS_XML.write().await;
	*rss_xml = channel.to_string();
	Ok(())
}

pub async fn get_rss_xml(mut tx: Tx<Postgres>) -> (StatusCode, String) {
	if RSS_XML.read().await.is_empty() && update_rss_xml(&mut tx).await.is_err() {
		print_and_ret!("Couldn't update index.xml")
	}

	(StatusCode::OK, RSS_XML.read().await.clone())
}
