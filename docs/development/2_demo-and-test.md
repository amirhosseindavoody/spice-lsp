# Demo and Testing

How to manually demo spice-lsp and automate tests at each layer.

## Testing pyramid

```
                    ┌─────────────┐
                    │  Manual VS  │  F5 extension, eyeball squiggles
                    │  Code demo  │
                    └──────┬──────┘
               ┌───────────┴───────────┐
               │  LSP integration tests │  JSON-RPC over stdio
               └───────────┬───────────┘
          ┌────────────────┴────────────────┐
          │     Parser / grammar tests       │  Fixtures in test-data/
          └─────────────────────────────────┘
```

Lower layers run faster and should carry most coverage.

---

## Parser tests

**Location:** `crates/spice-parser/tests/` or inline `#[cfg(test)]` modules.

**Pattern — golden diagnostics:**

```rust
#[test]
fn unclosed_subckt_reports_error() {
    let source = std::fs::read_to_string("test-data/invalid/unclosed-subckt.cir").unwrap();
    let result = spice_parser::analyze(&source);
    assert!(!result.diagnostics.is_empty());
    assert!(result.diagnostics[0].message.contains("ends"));
}
```

**Grammar tests (Tree-sitter):** Corpus files under `tree-sitter-spice/test/corpus/`:

```
==========
simple RC
==========
R1 in out 1k
---
(source_file (instance_line ...))
```

Run: `pixi run cargo test -p spice-parser`

---

## LSP integration tests

Test the binary without an editor by driving stdio.

### Option A — Custom test harness

Spawn `spice-lsp` as a child process, write JSON-RPC messages with Content-Length headers, read responses:

```rust
// Pseudocode
let mut child = Command::new("target/debug/spice-lsp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

write_message(&mut child, initialize_request());
let init_resp = read_message(&mut child);
assert_eq!(init_resp["result"]["capabilities"]["textDocumentSync"]["change"], 2);

write_message(&mut child, did_open("file:///test.cir", INVALID_SOURCE));
let diag = read_until_method(&mut child, "textDocument/publishDiagnostics");
assert!(!diag["params"]["diagnostics"].as_array().unwrap().is_empty());
```

### Option B — tower-lsp in-process tests

Test `Backend` methods directly with a mock `Client` that records `publish_diagnostics` calls — faster, no subprocess.

Use both: in-process for logic, one subprocess smoke test for the full binary.

Run: `pixi run cargo test -p spice-lsp`

---

## Manual LSP smoke test (no VS Code)

Use a generic LSP inspector or minimal script.

### With `languageclient` CLI (if installed)

Some ecosystems ship an inspector; alternatively use the VS Code **Output → SPICE LSP** trace.

### Raw JSON-RPC with Python (example)

```python
#!/usr/bin/env python3
"""Send initialize to spice-lsp over stdio. Requires built binary on PATH."""
import json, subprocess, struct, sys

proc = subprocess.Popen(
    ["spice-lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
)

def send(msg):
    body = json.dumps(msg).encode()
    header = f"Content-Length: {len(body)}\r\n\r\n".encode()
    proc.stdin.write(header + body)
    proc.stdin.flush()

def read():
    headers = {}
    while True:
        line = proc.stdout.readline().decode()
        if line in ("\r\n", "\n", ""):
            break
        k, v = line.split(":", 1)
        headers[k.strip()] = int(v.strip())
    body = proc.stdout.read(headers["Content-Length"])
    return json.loads(body)

send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
    "processId": None,
    "rootUri": None,
    "capabilities": {},
}})
resp = read()
print(json.dumps(resp, indent=2))
assert "capabilities" in resp.get("result", {}), resp
print("OK: initialize succeeded", file=sys.stderr)
```

Run after build:

```bash
pixi run build
export PATH="$PWD/target/debug:$PATH"
python3 scripts/lsp_smoke.py
```

Add `scripts/lsp_smoke.py` to the repo when the server exists.

---

## VS Code extension demo

### Development host (primary demo path)

1. Build Rust binary: `pixi run build`
2. `cd editors/vscode && npm install && npm run compile`
3. Open `editors/vscode` in VS Code
4. **Run and Debug → Launch Extension** (F5)
5. In the new **[Extension Development Host]** window:
   - Open `test-data/invalid/unclosed-subckt.cir`
   - Confirm **Problems** panel lists diagnostics
   - Fix syntax, confirm clearing

### Launch configuration

`.vscode/launch.json` in the extension folder:

```json
{
  "version": "0.2.0",
  "configurations": [{
    "name": "Launch Extension",
    "type": "extensionHost",
    "request": "launch",
    "args": ["--extensionDevelopmentPath=${workspaceFolder}"],
    "env": {},
    "preLaunchTask": "npm: compile"
  }]
}
```

Set user/workspace setting in Development Host:

```json
{
  "spiceLsp.serverPath": "/absolute/path/to/spice-lsp/target/debug/spice-lsp"
}
```

### Trace LSP traffic

Enable verbose logging during demo debugging:

```json
{
  "spiceLsp.trace.server": "verbose"
}
```

View **Output** panel → channel **SPICE LSP** (or Language Client name).

### Side-load packaged extension

```bash
cd editors/vscode
npx vsce package
code --install-extension spice-lsp-0.1.0.vsix
```

---

## Demo checklist for stakeholders

Use this script in reviews:

| Step | Action | Expected |
|------|--------|----------|
| 1 | `pixi run test` | All tests pass |
| 2 | F5 extension | Development Host opens |
| 3 | Open invalid netlist | Red squiggle + Problems entry |
| 4 | Edit to fix | Diagnostic disappears |
| 5 | Open valid netlist | No errors |
| 6 | **SPICE LSP: Restart Server** (if command added) | Server reconnects, diagnostics return |

---

## CI expectations

Every push should run:

```bash
pixi install
pixi run test
```

Optional nightly or pre-release:

```bash
pixi run build --release
pixi run cargo test --release
```

Extension CI (when added):

```bash
cd editors/vscode && npm ci && npm run compile && npm test
```

---

## Benchmarks (post-MVP)

Add `criterion` benches for parse + analyze on large fixtures:

```bash
pixi run cargo bench -p spice-parser
```

Track regressions against targets in [Architecture](../4_architecture.md).

---

## Related

- [VS Code integration](3_vscode-integration.md)
- [Build](1_build.md)
