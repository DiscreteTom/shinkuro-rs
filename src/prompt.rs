use crate::model::PromptData;
use crate::formatter::Formatter;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug)]
pub struct MarkdownPrompt {
    pub name: String,
    pub title: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
    pub content: String,
    pub arg_defaults: HashMap<String, String>,
    formatter: Formatter,
}

impl MarkdownPrompt {
    pub fn from_prompt_data(
        data: PromptData,
        formatter: Formatter,
        auto_discover: bool,
    ) -> Result<Self> {
        let (arguments, arg_defaults) = if auto_discover {
            if !data.arguments.is_empty() {
                anyhow::bail!("prompt_data.arguments must be empty when auto_discover_args is enabled");
            }
            let discovered = formatter.extract_arguments(&data.content)?;
            let mut args: Vec<_> = discovered.into_iter().collect();
            args.sort();
            (args.into_iter().map(|name| PromptArgument {
                name,
                description: String::new(),
                required: true,
            }).collect(), HashMap::new())
        } else {
            let discovered = formatter.extract_arguments(&data.content)?;
            let provided: std::collections::HashSet<_> = data.arguments.iter().map(|a| a.name.clone()).collect();
            if discovered != provided {
                anyhow::bail!("Content arguments {:?} don't match provided arguments {:?}", discovered, provided);
            }
            let mut defaults = HashMap::new();
            let args = data.arguments.into_iter().map(|a| {
                let required = a.default.is_none();
                if let Some(d) = a.default {
                    defaults.insert(a.name.clone(), d);
                }
                PromptArgument {
                    name: a.name,
                    description: a.description,
                    required,
                }
            }).collect();
            (args, defaults)
        };
        
        Ok(Self {
            name: data.name,
            title: data.title,
            description: data.description,
            arguments,
            content: data.content,
            arg_defaults,
            formatter,
        })
    }
    
    pub fn render(&self, args: Option<HashMap<String, String>>) -> Result<String, String> {
        let mut render_args = self.arg_defaults.clone();
        if let Some(a) = args {
            render_args.extend(a);
        }
        
        for arg in &self.arguments {
            if arg.required && !render_args.contains_key(&arg.name) {
                return Err(format!("Missing required arguments: {{{}}}", arg.name));
            }
        }
        
        Ok(self.formatter.format(&self.content, &render_args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Argument;

    #[test]
    fn test_markdown_prompt_from_prompt_data() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test Prompt".to_string(),
            description: "Test description".to_string(),
            arguments: vec![Argument {
                name: "user".to_string(),
                description: "User name".to_string(),
                default: None,
            }],
            content: "Hello {user}".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();

        assert_eq!(prompt.name, "test");
        assert_eq!(prompt.title, "Test Prompt");
        assert_eq!(prompt.description, "Test description");
        assert_eq!(prompt.arguments.len(), 1);
        assert_eq!(prompt.arguments[0].name, "user");
        assert!(prompt.arguments[0].required);
        assert_eq!(prompt.content, "Hello {user}");
    }

    #[test]
    fn test_markdown_prompt_with_defaults() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "user".to_string(),
                description: "User name".to_string(),
                default: Some("guest".to_string()),
            }],
            content: "Hello {user}".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();

        assert!(!prompt.arguments[0].required);
        assert_eq!(prompt.arg_defaults.get("user"), Some(&"guest".to_string()));
    }

    #[test]
    fn test_markdown_prompt_render_simple() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![],
            content: "Hello world".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();
        let result = prompt.render(None).unwrap();

        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_markdown_prompt_render_with_arguments() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "name".to_string(),
                description: "Name".to_string(),
                default: None,
            }],
            content: "Hello {name}!".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        let result = prompt.render(Some(args)).unwrap();

        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_markdown_prompt_render_with_defaults() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "name".to_string(),
                description: "Name".to_string(),
                default: Some("World".to_string()),
            }],
            content: "Hello {name}!".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();
        let result = prompt.render(None).unwrap();

        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_markdown_prompt_render_override_default() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "name".to_string(),
                description: "Name".to_string(),
                default: Some("World".to_string()),
            }],
            content: "Hello {name}!".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        let result = prompt.render(Some(args)).unwrap();

        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_markdown_prompt_missing_required_argument() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "name".to_string(),
                description: "Name".to_string(),
                default: None,
            }],
            content: "Hello {name}!".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false).unwrap();
        let result = prompt.render(None);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing required arguments"));
    }

    #[test]
    fn test_markdown_prompt_auto_discover() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![],
            content: "Hello {user} from {project}".to_string(),
        };

        let prompt = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, true).unwrap();

        assert_eq!(prompt.arguments.len(), 2);
        let names: Vec<_> = prompt.arguments.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"user"));
        assert!(names.contains(&"project"));
    }

    #[test]
    fn test_markdown_prompt_auto_discover_with_args_error() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "user".to_string(),
                description: "User".to_string(),
                default: None,
            }],
            content: "Hello {user}".to_string(),
        };

        let result = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be empty when auto_discover_args is enabled"));
    }

    #[test]
    fn test_markdown_prompt_argument_mismatch() {
        let data = PromptData {
            name: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            arguments: vec![Argument {
                name: "user".to_string(),
                description: "User".to_string(),
                default: None,
            }],
            content: "Hello {name}".to_string(),
        };

        let result = MarkdownPrompt::from_prompt_data(data, Formatter::Brace, false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("don't match"));
    }
}

