# @destack/template-blank

Start a new Destack package from an empty package with one setting and one page.

## Overview

`@destack/template-blank` is the smallest Destack package: a package handle, one setting and one page, to copy and grow.

It has:

- `package`: the handle stacks install the package by, with no objects yet.
- `language`: a setting for the language of generated content, per user and overridable per space.
- `home`: the start page, which Home's rail opens and `destack dev` reloads on every save.

Install it, and resolve its setting:

```sh
destack install @destack/template-blank
destack setting language get
```
