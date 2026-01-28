use std::collections::{HashMap, HashSet};
use anyhow::Result;

#[derive(Clone, Debug)]
pub enum Formatter {
    Brace,
    Dollar,
}

impl Formatter {
    pub fn extract_arguments(&self, content: &str) -> Result<HashSet<String>> {
        match self {
            Formatter::Brace => extract_brace_args(content),
            Formatter::Dollar => extract_dollar_args(content),
        }
    }

    pub fn format(&self, content: &str, variables: &HashMap<String, String>) -> String {
        match self {
            Formatter::Brace => format_brace(content, variables),
            Formatter::Dollar => format_dollar(content, variables),
        }
    }
}

pub fn validate_variable_name(name: &str) -> bool {
    if name.is_empty() { return false; }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' { return false; }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn extract_brace_args(content: &str) -> Result<HashSet<String>> {
    let mut args = HashSet::new();
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                continue;
            }
            let mut name = String::new();
            let mut found_close = false;
            while let Some(c) = chars.next() {
                if c == '}' {
                    found_close = true;
                    break;
                }
                name.push(c);
            }
            if found_close && !name.is_empty() {
                if !validate_variable_name(&name) {
                    anyhow::bail!("Invalid variable name: {}", name);
                }
                args.insert(name);
            }
        }
    }
    Ok(args)
}

fn extract_dollar_args(content: &str) -> Result<HashSet<String>> {
    let mut args = HashSet::new();
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'$') {
                chars.next();
                continue;
            }
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if !name.is_empty() {
                if !validate_variable_name(&name) {
                    anyhow::bail!("Invalid variable name: {}", name);
                }
                args.insert(name);
            }
        }
    }
    Ok(args)
}

fn format_brace(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                result.push('{');
                continue;
            }
            let mut name = String::new();
            let mut found_close = false;
            while let Some(c) = chars.next() {
                if c == '}' {
                    found_close = true;
                    break;
                }
                name.push(c);
            }
            if found_close {
                if let Some(value) = variables.get(&name) {
                    result.push_str(value);
                } else {
                    result.push('{');
                    result.push_str(&name);
                    result.push('}');
                }
            } else {
                result.push('{');
                result.push_str(&name);
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                chars.next();
                result.push('}');
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn format_dollar(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'$') {
                chars.next();
                result.push('$');
                continue;
            }
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if !name.is_empty() {
                if let Some(value) = variables.get(&name) {
                    result.push_str(value);
                } else {
                    result.push('$');
                    result.push_str(&name);
                }
            } else {
                result.push('$');
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn get_formatter(format_type: &str) -> Result<Formatter> {
    match format_type {
        "brace" => Ok(Formatter::Brace),
        "dollar" => Ok(Formatter::Dollar),
        _ => anyhow::bail!("Unknown formatter: {}", format_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_variable_name_valid() {
        assert!(validate_variable_name("user"));
        assert!(validate_variable_name("_private"));
        assert!(validate_variable_name("var123"));
        assert!(validate_variable_name("CamelCase"));
    }

    #[test]
    fn test_validate_variable_name_invalid() {
        assert!(!validate_variable_name(""));
        assert!(!validate_variable_name("123"));
        assert!(!validate_variable_name("var-name"));
        assert!(!validate_variable_name("var name"));
        assert!(!validate_variable_name("var.name"));
    }

    #[test]
    fn test_brace_formatter_extract_arguments() {
        let formatter = Formatter::Brace;
        let args = formatter.extract_arguments("Hello {user} from {project}").unwrap();
        assert_eq!(args.len(), 2);
        assert!(args.contains("user"));
        assert!(args.contains("project"));
    }

    #[test]
    fn test_brace_formatter_extract_arguments_invalid() {
        let formatter = Formatter::Brace;
        let result = formatter.extract_arguments("Hello {123}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid variable name"));
    }

    #[test]
    fn test_brace_formatter_format() {
        let formatter = Formatter::Brace;
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "Alice".to_string());
        let result = formatter.format("Hello {user}!", &vars);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_brace_formatter_escape() {
        let formatter = Formatter::Brace;
        let vars = HashMap::new();
        let result = formatter.format("Use {{var}} for variables", &vars);
        assert_eq!(result, "Use {var} for variables");
    }

    #[test]
    fn test_dollar_formatter_extract_arguments() {
        let formatter = Formatter::Dollar;
        let args = formatter.extract_arguments("Hello $user from $project").unwrap();
        assert_eq!(args.len(), 2);
        assert!(args.contains("user"));
        assert!(args.contains("project"));
    }

    #[test]
    fn test_dollar_formatter_format() {
        let formatter = Formatter::Dollar;
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "Alice".to_string());
        let result = formatter.format("Hello $user!", &vars);
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_dollar_formatter_safe_substitute() {
        let formatter = Formatter::Dollar;
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "Alice".to_string());
        let result = formatter.format("Hello $user $missing", &vars);
        assert_eq!(result, "Hello Alice $missing");
    }

    #[test]
    fn test_get_formatter_brace() {
        let formatter = get_formatter("brace").unwrap();
        assert!(matches!(formatter, Formatter::Brace));
    }

    #[test]
    fn test_get_formatter_dollar() {
        let formatter = get_formatter("dollar").unwrap();
        assert!(matches!(formatter, Formatter::Dollar));
    }

    #[test]
    fn test_get_formatter_invalid() {
        let result = get_formatter("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown formatter"));
    }
}

