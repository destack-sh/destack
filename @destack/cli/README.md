# @destack/cli

Use Destack from the terminal: sign in, install packages and work with a space's objects.

## Overview

`@destack/cli` is `destack`, a kubectl-style command line: fixed commands for the session, packages and stacks, and a command per object type a space installs.

The terminal acts as a person in a space:

- `login`, `logout`, `whoami` and `switch`: the person signed in, and the space commands act in.

Packages and stacks:

- `install`: installs a package, following a release tag or staying on a version.
- `apply`, `plan`, `approve` and `reject`: the space's stack and its plans.
- `dev`, `build`, `publish` and `tag`: a package in a checkout.

Objects:

- `destack <nouns>`: lists a type's objects.
- `destack <noun> <id> [method]`: gets an object, or calls a method on it.

Sign in, install a package, and work with its objects:

```sh
destack login
destack switch work
destack install @acme/notebooks
destack notebook create --name Plans
destack notebooks --where 'name = Plans' --watch
```

## Reference

### Commands

Every command takes `--space` to act in another space, `--json` for machine-readable output, and `--help`.

| Command                       | What it does                                                     |
| ----------------------------- | ---------------------------------------------------------------- |
| `login`                       | signs in with a device code, or `--browser` or `--email`         |
| `logout`, `whoami`            | signs out, or shows the person and space                         |
| `switch <space>`              | sets the space commands act in                                   |
| `install <package>`           | installs `@scope/name`, at `@tag` or `@version`                  |
| `apply <checkout>`            | applies a checkout's stack to the space                          |
| `plan`, `approve`, `reject`   | shows, approves or rejects the plan an installation waits on     |
| `dev <checkout>`              | previews a checkout's package live against the space             |
| `build`, `publish <checkout>` | builds a checkout's package, or builds and publishes it          |
| `tag`                         | points a distribution tag at a release                           |
| `setting`, `settings`         | gets, sets, resets or lists setting values at a `--scope`        |
| `open <target>`               | opens a link or an object in a desktop window or the browser     |
| `self`                        | installs, starts, stops, updates or uninstalls this distribution |
| `completion <shell>`          | prints the completion script for bash, zsh or fish               |

### Object commands

Each type the space installs adds a plural and a singular, such as `notebooks` and `notebook`.
`installation:noun` picks one installation's type when two share a name.

| Form                                      | What it does                                  |
| ----------------------------------------- | --------------------------------------------- |
| `destack <nouns> --where <filter>`        | lists objects matching a `@destack/db` filter |
| `destack <nouns> --limit <n> --watch`     | lists a page, and follows changes             |
| `destack <noun> <id>`                     | gets one object                               |
| `destack <noun> create --<field> <value>` | creates an object, with a flag per field      |
| `destack <noun> <id> <method> --<field>`  | calls a method, with a flag per input field   |

A failed command prints one error and exits with 2 for a usage error, 4 when signed out, and 130 when interrupted.

## Examples

### Install a package

`install` follows a tag, or stays on a version.

```sh
destack install @acme/notebooks                         # follow latest
destack install @acme/notebooks@next                    # follow next
destack install @acme/notebooks@2026.10.0 --name notes  # stay on 2026.10.0
```

### Apply a stack

`apply` changes the space's stack, and `approve` decides a plan that waits.

```sh
destack apply ./stack --export space --yes
destack plan --watch
destack approve notes --plan 9a2e…
```

### Develop and publish a package

`dev` previews a checkout's package, and `publish` releases it under a tag.

```sh
destack dev ./@acme/notebooks
destack publish ./@acme/notebooks --tag latest
```

## Comparison

### kubectl

Plural listings, `--watch` and declarative `apply` follow kubectl.

| kubectl                          | `destack`                  | Difference                                             |
| -------------------------------- | -------------------------- | ------------------------------------------------------ |
| `kubectl get <kind>`             | `destack <nouns>`          | Types come from the space's packages, without plugins. |
| `--selector`, `--field-selector` | `--where <filter>`         | One filter language over any field and relation.       |
| `kubectl apply -f`               | `destack apply <checkout>` | Risky plans wait for approval.                         |
| kubeconfig contexts              | `switch`                   | The machine keeps the session and its tokens.          |
