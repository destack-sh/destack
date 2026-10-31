import { createSignal, NoHydration } from "solid-js";
import type { JSX } from "@solidjs/web";
import { Package } from "@destack/package";
import { island, IslandContext, IslandPage } from "./island.tsx";

/** The package rendering the page. */
const PAGE_PACKAGE = Package.parse({
    id: "package-019f7480-0000-7000-8000-0000000000c1",
    name: "@example/islands",
    version: "2026.9.0",
});

/** Count clicks from a starting count. */
export function Counter(properties: { readonly start: number }) {
    const [count, setCount] = createSignal(properties.start);

    return <button onClick={() => setCount(count() + 1)}>Count: {count()}</button>;
}

/** Show or hide its children under a label. */
export function Disclosure(properties: {
    readonly label: string;
    readonly children?: JSX.Element;
}) {
    const [isOpen, setIsOpen] = createSignal(false);

    return (
        <section>
            <button onClick={() => setIsOpen(!isOpen())}>
                {properties.label}: {isOpen() ? "open" : "closed"}
            </button>
            {properties.children}
        </section>
    );
}

/** The counter as an island of its module. */
const CounterIsland = island(Counter, "/src/islands/counter.tsx");

/** The disclosure as an island of its module. */
const DisclosureIsland = island(Disclosure, "/src/islands/disclosure.tsx");

/** Render a page whose counter alone hydrates. */
export function Page() {
    return (
        <NoHydration>
            <IslandContext value={new IslandPage(PAGE_PACKAGE)}>
                <main>
                    <h1>Static</h1>
                    <CounterIsland start={3} />
                    <DisclosureIsland label="More">
                        <p>
                            Static <b>text</b>
                        </p>
                    </DisclosureIsland>
                </main>
            </IslandContext>
        </NoHydration>
    );
}
