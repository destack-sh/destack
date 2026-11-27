import type { Example } from "@destack/package/declare";
import { Component } from "@destack/view/declare";
import type { JSX } from "@destack/view";
import { DIRECTORIES } from "./components.ts";

/** The components that draw on a canvas, which the test's page has none of. */
const CANVAS = "chart";

/** The component directories the package exports, canvases left out. */
const DRAWN = DIRECTORIES.filter((name) => !name.startsWith(CANVAS));

/** The examples of every component the package exports. */
export const EXAMPLES: readonly (readonly [string, Example<object, JSX.Element>])[] = (
    await Promise.all(
        DRAWN.map(async (name) => {
            // read the component's module as its exports
            const module: unknown = await import(`../../src/${name}/${name}.example.tsx`);

            return Object.entries(module ?? {})
                .filter((entry): entry is [string, Example<object, JSX.Element>] =>
                    isExample(entry[1]),
                )
                .map(([exported, example]) => [exported, example] as const);
        }),
    )
).flat();

/** The components the package declares, by their exported names. */
export const COMPONENTS: readonly (readonly [string, Component])[] = (
    await Promise.all(
        DRAWN.map(async (name) => {
            // read the component's module as its exports
            const module: unknown = await import(`../../src/${name}/index.ts`);

            return Object.entries(module ?? {}).filter((entry): entry is [string, Component] =>
                Component.is(entry[1]),
            );
        }),
    )
).flat();

/** Report whether an export is an example. */
function isExample(value: unknown): value is Example<object, JSX.Element> {
    return typeof value === "object" && value !== null && "of" in value && "render" in value;
}
