mod model;
mod loader;
pub mod formatter;
mod prompt;
mod mcp;

use clap::Parser;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "shinkuro-rs", about = "Universal prompt loader MCP server", version)]
struct Args {
    #[arg(long, env = "FOLDER")]
    folder: Option<String>,
    #[arg(long, env = "GIT_URL")]
    git_url: Option<String>,
    #[arg(long, env = "CACHE_DIR", default_value = "~/.shinkuro/remote")]
    cache_dir: String,
    #[arg(long, env = "AUTO_PULL")]
    auto_pull: bool,
    #[arg(long, env = "VARIABLE_FORMAT", default_value = "brace")]
    variable_format: String,
    #[arg(long, env = "AUTO_DISCOVER_ARGS")]
    auto_discover_args: bool,
    #[arg(long, env = "SKIP_FRONTMATTER")]
    skip_frontmatter: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let folder_path = loader::get_folder_path(
        args.folder.as_deref(),
        args.git_url.as_deref(),
        &args.cache_dir,
        args.auto_pull,
    )?;
    
    let formatter = formatter::get_formatter(&args.variable_format)?;
    let prompts = loader::scan_markdown_files(&folder_path, args.skip_frontmatter)?;
    
    let mut server = mcp::McpServer::new();
    for prompt_data in prompts {
        let prompt = prompt::MarkdownPrompt::from_prompt_data(
            prompt_data,
            formatter.clone(),
            args.auto_discover_args,
        )?;
        server.add_prompt(prompt);
    }
    
    server.run().await
}
