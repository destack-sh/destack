# VSCode Host Tests

This folder contains deterministic extension host smoke tests for the Destack VSCode extension.
The suite validates extension activation, command registration, and basic language feature wiring.

The fixture workspace includes `tools/mock-lsp.js` as a minimal stdio JSON RPC server.
That server is intentionally local and dependency free so tests can run without requiring a built Destack binary.
This mock is authored in plain Node JS on purpose so it can execute directly from fixture settings with `node tools/mock-lsp.js`.

These tests are not semantic compiler tests.
They only verify VSCode extension host behavior and process wiring.
