# Icon artwork

`agent-job-relay.vectorcraft` is the editable source of truth, redrawn with the VectorCraft CLI MCP. It contains a transparent 1024 × 1024 RGB artboard and 21 vector objects in three named layers: Relay arrows, Agent and Completed job. Two curved arrows connect the agent and completed job; no separate checkpoint dot is used. No fonts, linked files or raster artwork are required.

| File | Use |
|---|---|
| `agent-job-relay.vectorcraft` | Native VectorCraft source; edit this file first |
| `agent-job-relay.svg` | Responsive vector export used by the READMEs and embedded in the offline runtime/plugin |
| `agent-job-relay.png` | Transparent 1024 × 1024 RGBA export |
| `../../plugins/chatgpt/assets/icon.svg` | Identical SVG export for the marketplace plugin |

## Edit and export

Use the `vectorcraft-cli` MCP tools. Resolve the checkout root to an absolute path and substitute it for `<repo>` in these calls; the placeholder is not a literal path.

1. Call `open_file` with `{"path":"<repo>/assets/icons/agent-job-relay.vectorcraft"}`, then edit the named objects/layers. Preserve the transparent artboard and existing navy (`#09286B`), blue (`#087DFA`), green (`#00BE91`), gray (`#91A6CA`) and white palette unless intentionally redesigning the icon.
2. Call `save_file` with `{"compress":false,"preview":false}` to keep the native source readable in Git, then reopen that saved `.vectorcraft` file before exporting.
3. Call `export` with each of the following argument objects:

```json
{"format":"svg","path":"<repo>/assets/icons/agent-job-relay.svg","artboard":0,"options":{"responsive":true,"metadata":true,"objectIds":"layerNames","decimals":3}}
{"format":"png","path":"<repo>/assets/icons/agent-job-relay.png","artboard":0,"scale":1,"options":{"background":"transparent","antiAlias":"art"}}
{"format":"svg","path":"<repo>/plugins/chatgpt/assets/icon.svg","artboard":0,"options":{"responsive":true,"metadata":true,"objectIds":"layerNames","decimals":3}}
```

Inspect the PNG and SVG rendering, verify the PNG dimensions/alpha and compare the two SVG files byte for byte. Commit the native source and all three exports together; do not hand-edit generated SVGs or retouch the PNG separately. Rebuild the Rust executable for offline bundles because `src/deployment.rs` embeds the canonical SVG at compile time. Artwork regeneration requires VectorCraft; normal builds use the checked-in exports.
