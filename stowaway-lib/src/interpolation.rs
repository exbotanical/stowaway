use crate::error::Result;
use std::collections::HashMap;

pub trait Interpolator {
    fn interpolate(&self, content: &str, variables: &HashMap<String, String>) -> Result<String>;
}

pub struct VariableInterpolator;

impl Interpolator for VariableInterpolator {
    fn interpolate(&self, content: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = content.to_string();

        for (key, value) in variables {
            let pattern = format!("@{}@", key);
            result = result.replace(&pattern, value);
        }
        // TODO: Return result with missing vars
        Ok(result)
    }
}

impl Default for VariableInterpolator {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let interpolator = VariableInterpolator;
        let mut variables = HashMap::new();
        variables.insert("username".to_string(), "testuser".to_string());
        variables.insert("editor".to_string(), "vim".to_string());

        let content = "User: @username@, Editor: @editor@";
        let result = interpolator.interpolate(content, &variables).unwrap();

        assert_eq!(result, "User: testuser, Editor: vim");
    }

    #[test]
    fn test_no_variables() {
        let interpolator = VariableInterpolator;
        let variables = HashMap::new();

        let content = "No variables here";
        let result = interpolator.interpolate(content, &variables).unwrap();

        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_missing_variable() {
        let interpolator = VariableInterpolator;
        let variables = HashMap::new();

        let content = "Missing: @missing@";
        let result = interpolator.interpolate(content, &variables).unwrap();

        assert_eq!(result, "Missing: @missing@");
    }

    #[test]
    fn test_multiple_same_variable() {
        let interpolator = VariableInterpolator;
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "test".to_string());

        let content = "@name@ and @name@ again";
        let result = interpolator.interpolate(content, &variables).unwrap();

        assert_eq!(result, "test and test again");
    }
}
