# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- MCP lifecycle: properly handle `notifications/initialized` without sending response
- Path expansion: resolve `~` and relative paths in `FOLDER` environment variable
- Trim leading and trailing whitespace from prompt content

## [0.1.1] - 2026-01-28

### Fixed

- Add musl targets (x86_64 and aarch64) for better Linux compatibility with older glibc versions

## [0.1.0] - 2026-01-28

### Added

- Initial Rust implementation of Shinkuro MCP server
- Support for loading markdown files from local folders
- Support for loading markdown files from git repositories (HTTPS and SSH)
- Frontmatter parsing for prompt metadata (name, title, description, arguments)
- Variable substitution with brace (`{var}`) and dollar (`$var`) formats
- Auto-discovery of template variables
- MCP protocol 2025-06-18 support
- Comprehensive test suite (32 tests)
- npm package distribution via cargo-dist

[unreleased]: https://github.com/DiscreteTom/shinkuro-rs/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/DiscreteTom/shinkuro-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/DiscreteTom/shinkuro-rs/releases/tag/v0.1.0
