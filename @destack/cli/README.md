# @destack/cli

Use Destack from the terminal: sign in, install packages and work with a space's objects.

## Overview

`destack` calls the objects of a space, its account and this computer through this computer's machine.

- **Session**: who is signed in and which space commands act on; `login`, `logout`, `whoami` and `switch`.
- **Noun**: an object type, its plural listing objects and its singular getting, creating and calling methods on one.
- **Installation**: a package installed in a space, following a release tag or staying on a version; `install`.
- **Stack**: the declared packages and resources of a space; `apply`, `plan`, `approve` and `reject`.
- **Package**: a package in a checkout; `dev`, `build`, `publish` and `tag`.

```sh
destack login
destack switch work
destack install @acme/notebooks
destack notebook create --name Plans
destack notebooks --where 'name = Plans' --watch
destack logout
```

## Examples

### Sign in

`login` signs in with a device code by default, and `--email` or `--browser` sign in another way.

```sh
destack login                           # a code for the printed page
destack login --email ada@example.com
destack login --browser                 # RFC 8252 loopback redirect
destack whoami
```

### Work with objects

A plural lists a type's objects and a singular calls a method on one, with a flag per field.

```sh
destack spaces
destack space create --name projects
destack space space-0199… rename --name archive
destack notes:notebooks                 # one installation's type
destack --space home tasks --limit 10
```

### Install a package

`install` follows a package's latest release or a tag, or stays on a version, and waits until the space serves it.

```sh
destack install @acme/notebooks                         # follows latest
destack install @acme/notebooks@next                    # follows next
destack install @acme/notebooks@2026.10.0 --name notes  # stays on 2026.10.0
```

### Apply a stack

`apply` changes the space's stack, and `approve` or `reject` decides a plan that waits on the space's managers.

```sh
destack apply ./stack --export space --yes
destack plan --watch
destack approve notes --plan 9a2e…
```

### Develop and publish a package

`dev` runs a package of a checkout in the current space, and `publish` releases it under a tag.

```sh
destack dev ./@acme/notebooks
destack build ./@acme/notebooks --outputs bun
destack publish ./@acme/notebooks --tag latest
```

### Update this computer's Destack

`self` starts, stops, updates and uninstalls this computer's Destack.

```sh
destack self start
destack self update
source <(destack completion zsh)
```

## Comparison

### kubectl

Plural listings, `--watch` and declarative `apply` follow kubectl; three things differ.

- Nouns and their methods come from the packages a space installs, so a package's types and verbs become commands without a plugin.
- A command runs through this computer's machine, which keeps the session and its tokens.
- A failed command prints one error with a code under `--json`, and exits with 2 for a usage error, 4 when signed out and 130 when interrupted.
