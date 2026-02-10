# Formatter Conformance Fixtures

This directory stores external formatter conformance corpora and baselines.
Parser conformance remains in `fixtures/conformance`.
Local formatter smoke fixtures live in `fixtures/formatter/smoke`.

## Structure

Fetched upstream suites are stored under:

- `conformance/staging/biome`
- `conformance/staging/prettier`
- `conformance/staging/oxfmt`

## Suggested suite sources

Use oxfmt as the hard external baseline for JS/TS behavior.

Use Biome and Prettier as advisory corpora for broader gap discovery.

Keep TS++ and annotation behavior in Destack owned fixtures under `transform` and `roundtrip`.

Treat Flow and `flow-repo` fixture trees as out of scope for Destack.
Keep those entries in `*-ignored.txt` rather than `*-known-failures.txt`.

See `SOURCES.md` for import priority and fixture mode policy.

## Fetch

Use the language justfile command:

```sh
just language/install-formatter-conformance
```

Or run scripts directly:

```sh
./biome-fetch.sh
./prettier-fetch.sh
./oxfmt-fetch.sh
```

Fetched source suites are stored under `conformance/staging/`.

The regular formatter test suite does not read `staging`.
Only the external harness reads these suites.
If you want a local curated case in the regular formatter suite, add it under `fixtures/formatter/smoke`.

The dedicated external suite harness reads directly from `conformance/staging/<suite>/`.

Run the external suite harness with:

```sh
just language/test-formatter-conformance
```

Known failures and ignored test lists live alongside this README.

- `biome-known-failures.txt`
- `biome-ignored.txt`
- `prettier-known-failures.txt`
- `prettier-ignored.txt`
- `oxfmt-known-failures.txt`
- `oxfmt-ignored.txt`
