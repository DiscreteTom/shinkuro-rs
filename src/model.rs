use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PromptData {
    pub name: String,
    pub title: String,
    pub description: String,
    pub arguments: Vec<Argument>,
    pub content: String,
}
