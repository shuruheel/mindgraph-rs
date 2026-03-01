# mindgraph Skill

A Claude Skill that teaches Claude the session lifecycle, research protocol,
and write discipline for the mindgraph persistent knowledge graph.

## Prerequisites

This skill requires the **mindgraph MCP server** to be running and connected
to Claude Desktop or Claude.ai. Set up the MCP first:

1. Build the binary: `cargo build -p mindgraph-mcp --release`
2. Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "mindgraph": {
      "command": "/absolute/path/to/target/release/mindgraph-mcp",
      "env": {
        "MINDGRAPH_SERVER_URL": "http://localhost:18790",
        "MINDGRAPH_API_KEY": "your-key-here"
      }
    }
  }
}
```

3. Start the mindgraph server:

```bash
MINDGRAPH_DB_PATH=/path/to/mindgraph.db MINDGRAPH_TOKEN=your-key-here \
  cargo run -p mindgraph-server --release
```

## Install the Skill

1. Zip this folder:
   ```bash
   zip -r mindgraph-skill.zip mindgraph-skill/
   ```
2. Open **Claude.ai** or **Claude Desktop** → Settings → Capabilities → Skills
3. Upload `mindgraph-skill.zip`

The skill activates automatically when the mindgraph MCP tools are available.

## Compatibility

Works across Claude.ai, Claude Code, and the API on Pro, Max, Team, and
Enterprise plans.
