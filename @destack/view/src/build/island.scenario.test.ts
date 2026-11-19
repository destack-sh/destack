import { expect, test } from "@destack/test";
import { wrapIsland } from "./island.ts";

test("wrap a named island component as the default island", () => {
    const code = [
        `import { createSignal } from "@destack/view";`,
        `export default function Counter(properties: { readonly start: number }) {`,
        `    return <button>{properties.start}</button>;`,
        `}`,
    ].join("\n");

    expect(wrapIsland(code, "counter.tsx", "/src/islands/counter.tsx").code.split("\n")).toEqual([
        `import { island } from "@destack/view/static";`,
        `import { createSignal } from "@destack/view";`,
        `function Counter(properties: { readonly start: number }) {`,
        `    return <button>{properties.start}</button>;`,
        `}`,
        `export default island(Counter, "/src/islands/counter.tsx");`,
        ``,
    ]);
});

test("bind an exported island expression as the component", () => {
    expect(
        wrapIsland(
            `export default () => <p>hi</p>;`,
            "hello.tsx",
            "/src/islands/hello.tsx",
        ).code.split("\n"),
    ).toEqual([
        `import { island } from "@destack/view/static";`,
        `const component = () => <p>hi</p>;`,
        `export default island(component, "/src/islands/hello.tsx");`,
        ``,
    ]);
});
