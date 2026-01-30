use crate::prompt::MarkdownPrompt;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorObject>,
}

#[derive(Serialize)]
struct ErrorObject {
    code: i32,
    message: String,
}

pub struct McpServer {
    prompts: HashMap<String, MarkdownPrompt>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    pub fn add_prompt(&mut self, prompt: MarkdownPrompt) {
        self.prompts.insert(prompt.name.clone(), prompt);
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            if let Ok(req) = serde_json::from_str::<Request>(&line) {
                if let Some(resp) = self.handle_request(req) {
                    let json = serde_json::to_string(&resp)?;
                    stdout.write_all(json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                }
            }
            line.clear();
        }
        Ok(())
    }

    fn handle_request(&self, req: Request) -> Option<Response> {
        match req.method.as_str() {
            "initialize" => Some(Response {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": { "prompts": {} },
                    "serverInfo": { "name": "shinkuro", "version": env!("CARGO_PKG_VERSION") }
                })),
                error: None,
            }),
            "notifications/initialized" => None,
            "prompts/list" => Some(Response {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({
                    "prompts": self.prompts.values().map(|p| json!({
                        "name": p.name,
                        "title": p.title,
                        "description": p.description,
                        "arguments": p.arguments.iter().map(|a| json!({
                            "name": a.name,
                            "description": a.description,
                            "required": a.required
                        })).collect::<Vec<_>>()
                    })).collect::<Vec<_>>()
                })),
                error: None,
            }),
            "prompts/get" => {
                let name = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str());

                if let Some(name) = name {
                    if let Some(prompt) = self.prompts.get(name) {
                        let args = req
                            .params
                            .as_ref()
                            .and_then(|p| p.get("arguments"))
                            .and_then(|a| {
                                serde_json::from_value::<HashMap<String, String>>(a.clone()).ok()
                            });

                        match prompt.render(args) {
                            Ok(content) => Some(Response {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(json!({
                                    "messages": [{ "role": "user", "content": { "type": "text", "text": content } }]
                                })),
                                error: None,
                            }),
                            Err(e) => Some(Response {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: None,
                                error: Some(ErrorObject {
                                    code: -32602,
                                    message: e,
                                }),
                            }),
                        }
                    } else {
                        Some(Response {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: None,
                            error: Some(ErrorObject {
                                code: -32602,
                                message: "Prompt not found".to_string(),
                            }),
                        })
                    }
                } else {
                    Some(Response {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: None,
                        error: Some(ErrorObject {
                            code: -32602,
                            message: "Missing name parameter".to_string(),
                        }),
                    })
                }
            }
            _ => Some(Response {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(ErrorObject {
                    code: -32601,
                    message: "Method not found".to_string(),
                }),
            }),
        }
    }
}
