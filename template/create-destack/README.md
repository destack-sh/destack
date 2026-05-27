# create-destack

Create a new Destack app.
This package powers `npm create destack`.

## Usage

```sh
npm create destack@latest my-app
```

```sh
pnpm create destack@latest my-app
```

```sh
yarn create destack my-app
```

```sh
bun create destack my-app
```

The default template is `app`.
Use `--template blank` for a minimal starter.

## Options

```sh
npm create destack@latest my-app -- --template app --yes
```

- `-t, --template <name>`: Select `app` or `blank`.
- `-p, --package-manager <name>`: Select `npm`, `pnpm`, `yarn`, or `bun`.
- `--overwrite`: Allow writing into a non-empty target directory.
- `--dry-run`: Print the planned actions without writing files.
- `-y, --yes`: Skip prompts.
- `-h, --help`: Show CLI help.

## Testing

Run these from the repository root.

```sh
# focused local loop
just check

# clean check
just check-quick

# exhaustive check
just check-full
```
