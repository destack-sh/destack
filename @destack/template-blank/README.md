# @destack/template-blank

Start a new Destack package from an empty package with one setting.

## Overview

`@destack/template-blank` is the smallest Destack package: a package handle and one setting, to copy and grow.

It has:

- `package`: the handle stacks install the package by, with no objects yet.
- `language`: a setting for the language of generated content, per user and overridable per space.

Install it, and resolve its setting:

```sh
destack install @destack/template-blank
destack setting language get
```
