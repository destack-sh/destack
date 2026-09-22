Render interfaces with Solid 2.

## Usage

```ts
import { createSignal } from "@destack/view";
import { render } from "@destack/view/render";
```

## Router

Define view routes with Solid Router 2.

```tsx
import { createRouter, defineRoutes } from "@destack/view/router";

const routes = defineRoutes([
    { path: "/", component: () => <h1>Home</h1> },
]);
export const Router = createRouter({ routes });
```

## Document

Declare HTML metadata with Solid Meta 1.

```tsx
import { Head, Meta, Title } from "@destack/view/document";

export function Metadata() {
    return (
        <Head>
            <Title>Notes</Title>
            <Meta name="description" content="Your notes." />
        </Head>
    );
}
```

## Conventions

Use view primitives inside components and standalone signals in shared application models. Build JSX
with `@destack/build`; `@destack/view/runtime` supplies compiler instructions. Import reactive
diagnostics from `@destack/view/inspect`.
