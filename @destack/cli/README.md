# @destack/cli

Use Destack from the terminal: sign in, install packages and work with a space's objects.

## Overview

`@destack/cli` is `destack`, a kubectl-style command line: a command per object type the space installs, and a few verbs for the journeys a person takes.
Every line it prints follows one visual language, which [Conventions](#conventions) lists.

Sign in, install a package, and work with its objects:

```sh
destack login
destack switch work
destack install @acme/notebooks
destack notebook new --name Plans
destack notebooks --where 'name = Plans' --watch
```

## Reference

### Grammar

Each type the space installs adds a plural and a singular, such as `notebooks` and `notebook`.
`installation:noun` picks one installation's type when two share a name.

| Form                                     | What it does                                  |
| ---------------------------------------- | --------------------------------------------- |
| `destack <nouns> --where <filter>`       | lists objects matching a `@destack/db` filter |
| `destack <nouns> --limit <n> --watch`    | lists a page, and follows changes             |
| `destack <noun> <id>`                    | gets one object, by its identifier or its key |
| `destack <noun> new --<field> <value>`   | creates an object, with a flag per field      |
| `destack <noun> <id> <method> --<field>` | calls a method, with a flag per input field   |

### Verbs

`destack` without words lists the verbs by journey, as this table does.

| Journey  | Verbs                                                         |
| -------- | ------------------------------------------------------------- |
| you      | `login`, `logout`, `whoami`, `self`, `completion`             |
| spaces   | `switch`                                                      |
| apps     | `install`, `uninstall`, `open`, `apply`, `plan`, `approve`, `reject` |
| share    | `share`, `invited`, `accept`                                  |
| build    | `new`, `dev`, `build`, `publish`, `tag`                       |
| settings | `setting`, `settings`                                         |

### Flags

Every command takes `--space`, `--json` and `--help`, and a command that cannot be undone takes `--yes`.

```sh
destack notebooks --space work--acme        # act in another space
destack notebooks --json                    # print JSON, one value a line when following
destack notebook plans delete --yes         # delete without the question
```

### Exit codes

A failed command prints one error and exits with a status.

| Status | Means                    |
| ------ | ------------------------ |
| 1      | the command failed       |
| 2      | the command was misused  |
| 4      | nobody is signed in      |
| 130    | the command was stopped  |

## Examples

### Install a package

`install` follows a tag, or stays on a version, and `uninstall` removes the installation with its resources.

```sh
destack install @acme/notebooks                         # follow latest
destack install @acme/notebooks@next                    # follow next
destack install @acme/notebooks@2026.10.0 --name notes  # stay on 2026.10.0
destack uninstall @acme/notebooks
```

### Apply a stack

`apply` changes the space's stack, and `approve` decides a plan that waits.

```sh
destack apply ./stack --export space --yes
destack plan --watch
destack approve notes --plan 9a2e…
```

### Develop and publish a package

`new` makes a package from a template, `dev` previews it, and `publish` releases it under a tag.

```sh
destack new @acme/notebooks
destack dev ./notebooks
destack publish ./notebooks --tag latest
```

## Conventions

### Lines

A command prints five kinds of line: an event, a question, a hint, a heading and a record.
An event, a question and a hint put one word in a gutter of ten characters, so a session's words line up.

```console
$ destack publish
  Building  package '@acme/notebooks' 3 outputs
 Published  release '@acme/notebooks@2026.11.1'
      next  destack install @acme/notebooks

$ destack notebook plans delete
         ?  Delete notebook 'plans'? This cannot be undone. yes/no
   Deleted  notebook 'Plans'

$ destack notebook missing
     error  notebook 'missing' not found
```

### Records

A table lists the name first and the identifier last, a count closes it, and one object opens with its heading.

```console
$ destack notebooks
  TITLE  OWNER  ID
  Plans  ada    notebook-01a2f04e-7c3b-71d0-9a5e-2f6c8d1b3e90
  Ideas  grace  notebook-01a31b7a-1d2e-7f40-8c6b-5a9e0d3f7c12

  2 notebooks

$ destack notebook plans
  Plans  notebook-01a2f04e-7c3b-71d0-9a5e-2f6c8d1b3e90

  owner  ada
```

### Colours

Each treatment has one meaning, and a state is always a word or a glyph too.

| Treatment | Means                                             |
| --------- | ------------------------------------------------- |
| orange    | under way, or where a person acts                 |
| green     | done, or ready                                    |
| red       | failed                                            |
| dim       | secondary: identifiers, labels, details and hints |
| bold      | the heading of one object                         |

### States

A list marks how each row stands with a dot.

| Dot | Means    |
| --- | -------- |
| `●` | ready    |
| `◐` | changing |
| `○` | off      |
| `✕` | failed   |

### Wording

An event names its noun and quotes its name, and its word's tense says how it stands.

```console
  Building  package '@acme/notebooks'
 Published  release '@acme/notebooks@2026.11.1'
     error  cannot uninstall 'notebooks': it deletes the installation's resources, so confirm it with --yes
```

### Programs

A pipe gets the same words without the gutter, the margin and the colour, a table's rows tab-separated without a header, and `--json` the values.

```console
$ destack notebooks | cat
Plans	ada	notebook-01a2f04e-7c3b-71d0-9a5e-2f6c8d1b3e90
Ideas	grace	notebook-01a31b7a-1d2e-7f40-8c6b-5a9e0d3f7c12
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

### cargo

The gutter of an event follows cargo's right-aligned status words.

| cargo                 | `destack`                 | Difference                                           |
| --------------------- | ------------------------- | ---------------------------------------------------- |
| `   Compiling`, green | `  Building`, by state    | Questions, hints and errors share the same gutter.   |
| `error:` at the edge  | `error` in the gutter     | Causes and the trace follow as hints below it.       |
