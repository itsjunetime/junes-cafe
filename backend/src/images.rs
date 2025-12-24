use axum::http::StatusCode;
use tower_sessions::Session;
use std::time::{SystemTime, UNIX_EPOCH};
use backend::check_auth;

use crate::print_and_ret;

pub async fn upload_asset(session: Session, mut form: multer::Multipart<'_>) -> (StatusCode, String) {
	check_auth!(session);

	let mut name = None;

	// We need to loop over each field of the form
	loop {
		match form.next_field().await {
			Ok(field_opt) => {
				// If it doesn't exist, we've exhausted all the fields
				let Some(field) = field_opt else {
					break;
				};

				if field.name() == Some("name") {
					name = field.text().await.ok();
					continue;
				}

				if field.name() != Some("file") {
					continue;
				}

				// Just use the current time as the name of the file. Ideally we'd like sha256 hash
				// the data or whatever, but I'm too lazy to do that.
				let file_name = match SystemTime::now().duration_since(UNIX_EPOCH) {
					Ok(t) => t.as_nanos().to_string(),
					Err(e) => print_and_ret!("Couldn't get current time: {e:?}"),
				};

				// And then make sure we can actually get the data of the file
				let asset_data = match field.bytes().await {
					Ok(b) if !b.is_empty() => b,
					Err(e) => print_and_ret!("Couldn't get asset data; you may have exceeded the 10mb limit ({e:?})"),
					_ => print_and_ret!(StatusCode::BAD_REQUEST, "Sent an empty asset")
				};

				// And create the file to save it at
				let asset_dir = match dotenv::var("ASSET_DIR") {
					Ok(d) => d,
					Err(e) => print_and_ret!("Couldn't get ASSET_DIR: {e:?}"),
				};

				let asset_path = std::path::Path::new(&asset_dir);
				let mut save_path = asset_path.join(&file_name);

				if let Some(ext) = name.as_ref().and_then(|n| n.split('.').next_back()) {
					_ = save_path.set_extension(ext);
				}

				let res = std::fs::write(&save_path, asset_data)
					.map_or_else(
						|e| print_and_ret!("Couldn't save the asset to {save_path:?}: {e:?}"),
						|()| {
							let path = save_path.file_name()
								.and_then(|s| s.to_os_string().into_string().ok())
								.unwrap();

							// let asset_path = format!("/api/assets/{path}");
							// inval.invalidate_all_with_pred(|(_, uri)| uri.path() == asset_path);

							(StatusCode::OK, path)
						}
					);

				if name.is_some_and(|name| name.ends_with("png")) {
					tokio::spawn(async move {
						// optimize it on some background thread if it's a png
						if let Err(e) = oxipng::optimize(
							&oxipng::InFile::Path(save_path),
							&oxipng::OutFile::Path {
								// just use the same as InFile
								path: None,
								// we want to keep the same permissions
								preserve_attrs: true
							},
							&oxipng::Options {
								fix_errors: true,
								deflater: oxipng::Deflater::Libdeflater { compression: 12 },
								..Default::default()
							}
						) {
							eprintln!("Couldn't optimize uploaded file: {e}");
						}
					});
				}

				return res;
			},
			Err(err) => print_and_ret!("Couldn't get all fields of request: {err:?}")
		}
	}

	(StatusCode::BAD_REQUEST, "Form didn't contain the requisite 'file' field".into())
}
