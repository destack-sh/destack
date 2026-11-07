# @destack/cli

`destack` reads and changes the objects of a space, its account and this computer from the terminal, through this computer's machine.

```sh
destack login                                     # a device code, switching to your home
destack install @acme/notebooks                   # the latest release, in the current space
destack switch work                               # an address, an alias, home, residency or a space's name
destack notebooks --where 'name = Plans' --watch  # list, redrawn as the rows change
destack notebook notebook-0199… update --name Ideas --revision 2  # a method call with field flags
destack notebook create --name Plans
destack notes:notebooks                           # a noun two installations serve, qualified by the installation
destack logs --where 'severity >= 17' --watch     # the space's events as a noun
destack setting palette.shortcut set alt+k --scope account
destack apply ./stack --export space --yes
destack open notebook-0199…                       # in a desktop window
```

## Context

`login` signs you in through this computer's machine, which stores the session and enrolls itself in your account.

```sh
# RFC 8628: a one-time code to enter at the printed page in any browser, which also signs people up
destack login
# sign in, asking for the code alone
destack login --email ada@example.com
# scripted: the code on standard input
DESTACK_EMAIL=ada@example.com destack login --json < code.txt
# RFC 8252: the browser and a loopback redirect
destack login --browser
# the person, their session and the space switched to
destack whoami
# another space for one command line: an address, an alias of space.aliases, home, residency or a name
destack --space home spaces
destack logout
```

## Objects

A plural lists a type's objects, and a singular gets, creates and calls methods on one, from the nouns the stores of this computer's machine, the universe and the space describe.

```sh
destack spaces                                       # account:spaces, served by the universe
destack space create --name projects --machine machine-0199…
destack space --help                                 # the methods of one space
destack space space-0199… rename --name archive
destack calls --watch                                # object:calls, the space's call log
destack issues --where 'status = unresolved'         # where observability is installed
destack --space residency charges --status open      # finance, in your account's residency
destack setting start set destack://home--home--ada/home --scope device   # this computer's value
destack settings --scope space
```

## Commands

A command a package declares on its type runs on one object as a verb, setting its input and refused where its `when` misses.

```sh
# update's restart-to-update: distribution.activate, offered once a release is staged
destack distribution distribution-0199… restart-to-update
# a notebook command presetting its input, the other fields as flags
destack notebook notebook-0199… rename-to-ideas --revision 3
# {"error":{"code":"PRECONDITION_FAILED","message":"rename-to-ideas does not apply to notebook notebook-0199…"}}
```

## Stacks

`apply`, `plan` and `approve` change the space's stack, asking before a plan applies.

```sh
destack apply ./stack --export space --parameters '{"region":"eu"}'
destack plan --watch
destack approve --plan 3f1c…
```

## Installations

`install` installs a package's release in the space and waits until the space serves it.

```sh
destack install @acme/notebooks@2026.10.0 --name notes
# the same through the installation type, a flag per field of the selection
destack installation create --package-id package-0199… --selection-kind release --selection-version 2026.10.0 --name notes
```

## Packages

`dev`, `build` and `publish` run, build and publish a package of a checkout.

```sh
destack checkout create --directory ~/code/notebooks   # machine:checkout, this computer's
destack dev ./@acme/notebooks
destack build ./@acme/notebooks --outputs bun
destack publish ./@acme/notebooks --tag latest
```

## Distribution

`self` installs, updates and uninstalls this computer's Destack, and `completion` prints a shell's completion script.

```sh
destack self start                     # start this computer's machine, or reuse the one running
destack self stop                      # stop this computer's machine
destack self update --stage            # distribution.stage: download and verify
# stage, activate, restart the machine and relaunch the desktop
destack self update
# stop the machine, wait for the desktop to quit, leave the state
destack self uninstall
source <(destack completion zsh)       # spaces for switch, a setting's values, directories for dev and apply
```

## Errors

A failed command exits with 2 for a usage error, 4 when nobody is signed in, 130 once interrupted, and 1 otherwise.

```sh
# {"error":{"code":"NOT_FOUND","message":"no space of yours is called notes"}}
destack --space notes whoami --json
# {"error":{"code":"BAD_REQUEST","message":"--limit takes a positive whole number"}}
destack tasks --limit 0 --json
```
