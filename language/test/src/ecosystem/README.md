## Ecosystem Tests

Ecosystem tests validate Destack against real-world TypeScript/JavaScript packages.

### Fixtures
- `fixtures/ecosystem/packages/*.toml`: package manifests (repo + pinned ref + discovery rules)
- `fixtures/ecosystem/cache/`: fetched package checkouts (not committed)
- `fixtures/ecosystem/patches/<package>/`: optional overlays applied before testing

### Fetching (fast by default)
Tests do not fetch packages automatically. Fetch them explicitly:

```bash
./language/test/fixtures/ecosystem/ecosystem-fetch.sh
```

### Running
```bash
cargo test -p destack_test --test ecosystem
cargo test -p destack_test --test ecosystem -- --list
cargo test -p destack_test --test ecosystem -- ms
cargo test -p destack_test --test ecosystem -- --analyze
```

