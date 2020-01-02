use std::env;
use std::env::VarError;

pub fn get_env_var_value(var_name: &str) -> Result<String, VarError> {
    env::var(var_name)
}

pub fn get_env_var_value_or_default(var_name: &str, default: Option<String>) -> String {
    get_env_var_value(var_name).unwrap_or(
        default.unwrap_or(
            "".to_string()
        )
    )
}