Render interfaces with Solid 2.

## Usage

```ts
import { createSignal } from "@destack/view";
import { render } from "@destack/view/render";
```

## Conventions

Use view primitives inside components and standalone signals in shared application models. Build JSX
with `@destack/build`; `@destack/view/runtime` supplies compiler instructions. Import reactive
diagnostics from `@destack/view/inspect`.
