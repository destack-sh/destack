# HTML Fixtures

These fixtures are sourced from strong upstream HTML parser corpora and kept checked in for stable local runs.

Refresh the html5lib corpus with:

```sh
./language/test/fixtures/html/fetch.sh
```

The custom regression fixtures under `tokenizer/` are local and are not overwritten by that fetch script.
The `tree-construction/scripted/` subtree is intentionally pruned because those cases require script execution rather than parser-only tree construction.
