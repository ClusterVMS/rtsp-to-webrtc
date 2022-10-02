use std::str::FromStr;
use url::Url;

pub const SRC_URL_ENV_VAR : &str = "CAM_SRC_URL";
pub const SRC_USERNAME_ENV_VAR : &str = "CAM_USERNAME";
pub const SRC_PASSWORD_ENV_VAR : &str = "CAM_PASSWORD";

pub struct StreamSettings {
	pub source_url: Url,
	pub username: String,
	pub password: String,
}

pub fn get_required_env_var(var_name: &str) -> String {
	std::env::var(var_name).expect(format!("Environment variable {var_name} should be set.").as_str())
}

pub fn get_src_stream_settings() -> StreamSettings {
	let source_url = Url::from_str(get_required_env_var(SRC_URL_ENV_VAR).as_str()).unwrap();
	let username = get_required_env_var(SRC_USERNAME_ENV_VAR);
	let password = get_required_env_var(SRC_PASSWORD_ENV_VAR);

	StreamSettings{
		source_url,
		username,
		password,
	}
}
