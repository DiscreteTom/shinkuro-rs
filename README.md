# Shinkuro-rs - Universal prompt loader MCP server (Rust)

Rust implementation of [Shinkuro](https://github.com/DiscreteTom/shinkuro) - loads markdown files from a local folder or git repository and serves them as [MCP Prompts](https://modelcontextprotocol.io/specification/2024-11-05/server/prompts).

## Features

- Loads markdown files as MCP prompts
- Supports frontmatter metadata (name, title, description, arguments)
- Template variable substitution (`{var}` or `$var`)
- Git repository cloning and caching
- Auto-discovery of template variables
- Lightweight manual MCP protocol implementation

## Usage

### Build

```bash
cargo build --release
```

Binary location: `target/release/shinkuro-rs`

### Local Files

```json
{
  "mcpServers": {
    "shinkuro-rs": {
      "command": "/path/to/shinkuro-rs",
      "env": {
        "FOLDER": "/path/to/prompts"
      }
    }
  }
}
```

### Remote Git Repository

```json
{
  "mcpServers": {
    "shinkuro-rs": {
      "command": "/path/to/shinkuro-rs",
      "env": {
        "GIT_URL": "https://github.com/owner/repo.git",
        "FOLDER": "prompts"
      }
    }
  }
}
```

## Options

- `--folder` / `FOLDER`: Path to local folder or subfolder within git repo
- `--git-url` / `GIT_URL`: Git repository URL
- `--cache-dir` / `CACHE_DIR`: Cache directory (default: `~/.shinkuro/remote`)
- `--auto-pull` / `AUTO_PULL`: Refresh cache on startup
- `--variable-format` / `VARIABLE_FORMAT`: `brace` (default) or `dollar`
- `--auto-discover-args` / `AUTO_DISCOVER_ARGS`: Auto-discover template variables
- `--skip-frontmatter` / `SKIP_FRONTMATTER`: Skip frontmatter processing
