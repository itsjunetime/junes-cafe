use axum::{
	extract::{Multipart, Path},
	http::StatusCode
};
use axum_sessions::extractors::ReadableSession;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{print_and_ret, check_auth, USERNAME_KEY};

pub async fn upload_image(session: ReadableSession, mut form: Multipart) -> (StatusCode, String) {
	check_auth!(session);

	// We need to loop over each field of the form
	loop {
		match form.next_field().await {
			Ok(field_opt) => {
				// If it doesn't exist, we've exhausted all the fields
				let Some(field) = field_opt else {
					break;
				};

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
				let image_data = match field.bytes().await {
					Ok(b) if !b.is_empty() => b,
					Err(e) => print_and_ret!("Couldn't get image data; you may have exceeded the 10mb limit ({e:?})"),
					_ => print_and_ret!(StatusCode::BAD_REQUEST, "Sent an empty image")
				};

				// And create the file to save it at
				let image_dir = match dotenv::var("IMAGE_DIR") {
					Ok(d) => d,
					Err(e) => print_and_ret!("Couldn't get IMAGE_DIR: {e:?}"),
				};

				let image_path = std::path::Path::new(&image_dir);
				let save_path = image_path.join(&file_name);

				return std::fs::write(&save_path, image_data)
					.map_or_else(
						|e| print_and_ret!("Couldn't save the image to {save_path:?}: {e:?}"),
						|()| (StatusCode::OK, file_name)
					);
			},
			Err(err) => print_and_ret!("Couldn't get all fields of request: {err:?}")
		}
	}

	(StatusCode::BAD_REQUEST, "Form didn't contain the requisite 'file' field".into())
}

pub async fn get_image(Path(image): Path<String>) -> Result<Vec<u8>, StatusCode> {
	// Make sure we know the parent directory
	let image_dir = match dotenv::var("IMAGE_DIR") {
		Ok(d) => d,
		Err(e) => {
			eprintln!("Couldn't get IMAGE_DIR when getting image {image}: {e:?}");
			return Err(StatusCode::INTERNAL_SERVER_ERROR);
		}
	};

	// And make sure we can get a full path out of the string they gave us
	let full_path = match std::path::Path::new(&image_dir).join(&image).canonicalize() {
		Ok(p) => p,
		Err(e) => {
			eprintln!("Couldn't canonicalize full path for {image}: {e:?}");
			return Err(StatusCode::INTERNAL_SERVER_ERROR);
		}
	};

	// And if the full path isn't still inside the IMAGE_DIR directory, that means they're
	// attempting directory traversal, so we shouldn't let the request continue.
	if !full_path.starts_with(&image_dir) {
		eprintln!("Directory traversal attempted (submitted '{image}', resolved to {full_path:?})");
		return Err(StatusCode::BAD_REQUEST);
	}

	// And then read the file and return information based on what we read
	std::fs::read(&full_path)
		.map_err(|e| {
				eprintln!("Can't read file at {full_path:?}: {e:?}");
				match e.kind() {
					// If it can't be found, we're just assuming they submitted a bad request,
					// since there shouldn't be any images referenced on the site that don't exist
					// on the fs somewhere
					std::io::ErrorKind::NotFound => StatusCode::BAD_REQUEST,
					_ => StatusCode::INTERNAL_SERVER_ERROR
				}
		})
}
