---
title: Surfaces
description: Surfaces.
---

# Surfaces

## Desktop

- single download includes the host, runtime, CLI, desktop shell, and initial packages.

- daemon service picks up / runs local host stuff
- closing a window leaves background work running

- home opens applications inside a shared shell or in separate windows.
- launcher entries address a space, application instance, and view

- local state lives under `~/.destack`; editable repositories can live anywhere
- `service/desktop` manages native windows and OS integration; `app/home` provides the application UI.

## Preferences

- personal system space stores their preferences.
- applications use shared theme tokens and named commands from `library/ui`.
- keybindings map keys to commands and can be overridden per user or device.
- themes use the same StyleX tokens across applications.

## Web

- browser clients connect to the same spaces and services as desktop clients
- applications provide interactive views, server-rendered pages, and static assets.
- Public routes expose selected operations and content under explicit access rules.

- sign-in and application navigation share one account system.

## Mobile

- TBD, not ready yet
- simple mobile app that wraps
