# @destack/home

Start from Home: see your spaces and their apps, approve their plans, search commands and change your settings.

## Overview

`@destack/home` is the app every person starts from: their spaces, a command palette, settings, and the universe's sign-in pages.

Each person's home space installs it:

- `home`: the account's spaces, their apps, and the plans waiting on their managers.
- `command`: the command palette over windows, commands, views, spaces and recent objects.
- `settings`: the person's settings, such as `start`, the link a new window opens.
- `start`, `sign-in`, `sign-up`, `consent` and `apps`: the universe's sign-in pages.

Install Home in another space, and set the link new windows open:

```sh
destack install @destack/home
destack setting start set destack://notes--personal--ada/home --scope device
```

## Comparison

### Spotlight

The command palette follows Spotlight's search over apps, windows and recent items.

| Spotlight          | `command`                 | Difference                                                 |
| ------------------ | ------------------------- | ---------------------------------------------------------- |
| installed apps     | the spaces' installations | A new app appears once a space installs it.                |
| actions on results | object commands           | Runs the commands packages declare, with a form for input. |
