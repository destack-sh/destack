# @destack/home

Start from Home: see your spaces and their apps, approve their plans, search commands and change your settings.

## Overview

Home is installed in the home space of every person's personal account, and is the first view the desktop opens.

- **Home view**: the account's spaces, each with the views its installations mount and the plans waiting on its managers.
- **Command palette**: the `command` view, which searches the person's windows, commands, views, spaces and recent objects and runs a command.
- **Settings view**: the person's settings, such as `start`, the link a new window opens.
- **Sign-in views**: the universe's `start`, `sign-in`, `sign-up`, `consent` and `apps` pages.

```ts
import { command, home, settings } from "@destack/home/view";
import { start } from "@destack/home/setting";

const views = [home, command, settings];
start.definition.default; // null
```

## Examples

### Install Home

Every personal account's home space installs Home, and another space can install it too.

```sh
destack install @destack/home
```

### Set the start link

`start` takes a `destack://` link, set for the person or for this computer alone.

```sh
destack setting start set destack://notes--personal--ada/home --scope device
```

## Comparison

### Spotlight

The command palette follows Spotlight's search over apps, windows and recent items; two things differ.

- Home lists what the person's spaces install, so a new app appears once a space installs it.
- The palette runs the commands packages declare on their object types, filling each command's input as a form.
