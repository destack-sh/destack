import { expect, test } from "@destack/test";
import { createRoot } from "solid-js";
import { createMutationObserver, mutationObserver } from "./mutation-observer.ts";

/** Wait for the observers to deliver their records. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

/** Append a child to each element. */
function append(...parents: Element[]): void {
    for (const parent of parents) {
        parent.append(document.createElement("span"));
    }
}

test("observe initial nodes with shared options, from the moment it starts", async () => {
    // observe two parents once started
    const parents = [document.createElement("div"), document.createElement("div")];
    const targets: Node[] = [];
    const { start, dispose } = createRoot((disposeRoot) => {
        const [, { start: begin, instance, isSupported }] = createMutationObserver(
            parents,
            { childList: true },
            (records) => targets.push(...records.map((record) => record.target)),
        );

        return { start: begin, dispose: disposeRoot, instance, isSupported };
    });
    start();
    append(...parents);
    await settle();
    dispose();

    expect(targets).toEqual(parents);
});

test("observe nodes with their own options, and nodes added later", async () => {
    // observe one parent's children and another's attributes, then add a third
    const children = document.createElement("div");
    const attributes = document.createElement("div");
    const later = document.createElement("div");
    const kinds: string[] = [];
    const { add, start, dispose } = createRoot((disposeRoot) => {
        const [observe, { start: begin }] = createMutationObserver(
            [
                [children, { childList: true }],
                [attributes, { attributes: true }],
            ],
            (records) => kinds.push(...records.map((record) => record.type)),
        );

        return { add: observe, start: begin, dispose: disposeRoot };
    });
    start();
    add(later, () => ({ attributes: true }));

    // change what each observes, and what it does not
    append(children, attributes);
    attributes.setAttribute("title", "observed");
    later.setAttribute("title", "observed");
    await settle();
    dispose();

    expect(kinds).toEqual(["childList", "attributes", "attributes"]);
});

test("stop delivering records when stopped or disposed, dropping pending ones", async () => {
    // stop with a record pending, and dispose another observer
    const parent = document.createElement("div");
    let count = 0;
    const roots = [0, 1].map(() =>
        createRoot((disposeRoot) => {
            const [, { start, stop }] = createMutationObserver(
                parent,
                { childList: true },
                () => count++,
            );

            return { start, stop, dispose: disposeRoot };
        }),
    );
    for (const root of roots) {
        root.start();
    }
    append(parent);
    roots[0]?.stop();
    roots[1]?.dispose();
    await settle();
    roots[0]?.dispose();

    expect(count).toBe(0);
});

test("observe the element a ref receives, until its owner is disposed", async () => {
    // observe through the ref, then dispose and mutate again
    const element = document.createElement("div");
    let count = 0;
    const dispose = createRoot((disposeRoot) => {
        mutationObserver({ childList: true }, () => count++)(element);

        return disposeRoot;
    });
    append(element);
    await settle();
    dispose();
    append(element);
    await settle();

    expect(count).toBe(1);
});
