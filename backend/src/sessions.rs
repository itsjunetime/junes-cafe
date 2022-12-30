use dashmap::DashMap;
use tower::{Layer, Service};

pub struct SessionLayer;

impl<S> Layer<S> for SessionLayer {
	type Service = 
}

pub struct SessionService {
	sessions: DashMap<String, String>
}
