import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, Errored, flush, type JSX } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import {
    Pane,
    PaneBar,
    PaneContent,
    PaneEmpty,
    PaneGroup,
    PaneLayout,
    type PaneGroupControl,
    type PaneMove,
    PaneTabs,
    usePaneGroup,
} from "./index.ts";

/** Render a group whose layout a test changes, counting the contents it builds. */
function renderGroup(start: PaneLayout, onMove?: (move: PaneMove) => void) {
    stubPopovers();
    const [layout, setLayout] = createSignal(start, { ownedWrite: true });
    const built: string[] = [];
    const { container } = render(() => (
        <PaneGroup
            layout={layout()}
            onLayoutChange={setLayout}
            {...(onMove === undefined ? {} : { onMove })}
            content={(item) => {
                built.push(item().id);

                return <input aria-label={item().value} />;
            }}
            fallback="nothing open"
        />
    ));
    flush();
    const change = (verb: (all: PaneLayout) => PaneLayout): void => {
        setLayout(verb);
        flush();
    };

    return { container, layout, change, built };
}

/** Press a key on an element. */
function press(element: Element | null, key: string): void {
    element?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
    flush();
}

test("build a content once, however its item and its pane move", () => {
    const { container, layout, change, built } = renderGroup(PaneLayout.of(["a", "b"], ["c"]));
    const input = container.querySelector<HTMLInputElement>("input[aria-label=a]");
    if (input !== null) {
        input.value = "typed";
    }
    const [one, two] = PaneLayout.panes(layout()).map(({ id }) => id);
    change((all) => PaneLayout.move(all, "a", { kind: "side", pane: one ?? "", side: "bottom" }));
    change((all) => PaneLayout.swap(all, one ?? "", two ?? ""));
    change((all) => PaneLayout.move(all, "a", { kind: "item", pane: two ?? "" }));
    const after = container.querySelector<HTMLInputElement>("input[aria-label=a]");

    expect({ isSame: after === input, value: after?.value, built }).toEqual({
        isSame: true,
        value: "typed",
        built: ["a", "c", "b"],
    });
});

test("select, close and empty by its tabs", () => {
    const { container, layout } = renderGroup(PaneLayout.of(["a", "b"]));
    const tab = (name: string) =>
        [...container.querySelectorAll<HTMLElement>("[role=tab]")].find(
            (each) => each.textContent === name,
        );
    tab("b")?.click();
    flush();
    const selected = PaneLayout.panes(layout()).map((pane) => PaneLayout.selected(pane)?.id);
    for (let count = 0; count < 2; count += 1) {
        container.querySelector<HTMLElement>("[data-slot=tabs-close]")?.click();
        flush();
    }

    expect({
        selected,
        text: container.querySelector("[data-slot=sortable]")?.textContent,
    }).toEqual({
        selected: ["b"],
        text: "nothing open",
    });
});

test("label a content by its tab, and nothing without tabs", () => {
    const { container } = renderGroup(PaneLayout.of(["a"]));
    const panel = container.querySelector("[role=tabpanel]");
    const labelled = panel?.getAttribute("aria-labelledby") ?? "";
    const [layout] = createSignal(PaneLayout.of(["a"]));
    const plain = render(() => (
        <PaneGroup layout={layout()} content={() => <p>text</p>}>
            {() => (
                <Pane>
                    <PaneBar>bar</PaneBar>
                    <PaneContent />
                </Pane>
            )}
        </PaneGroup>
    )).container;
    flush();

    expect({
        tab: container.ownerDocument.getElementById(labelled)?.textContent,
        isControlled:
            container.querySelector("[role=tab]")?.getAttribute("aria-controls") === panel?.id,
        plain: plain.querySelectorAll("[role=tabpanel]").length,
    }).toEqual({ tab: "a", isControlled: true, plain: 0 });
});

test("show an empty pane's own words while it stays", () => {
    stubPopovers();
    const start = PaneLayout.of(["a"]);
    const kept: PaneLayout = {
        ...start,
        root:
            start.root?.kind === "leaf"
                ? { ...start.root, isKept: true, items: [], recent: [] }
                : start.root,
    };
    const { container } = render(() => (
        <PaneGroup layout={kept}>
            {() => (
                <Pane>
                    <PaneTabs />
                    <PaneContent />
                    <PaneEmpty>drop something here</PaneEmpty>
                </Pane>
            )}
        </PaneGroup>
    ));
    flush();

    expect(container.querySelector("[data-slot=pane]")?.textContent).toBe("drop something here");
});

test("move the focus on Tab from a pane's tabs to its content, and on to the next pane", () => {
    const { container } = renderGroup(PaneLayout.of(["a", "b"], ["c"]));
    const focused: (string | null)[] = [];
    container.querySelector<HTMLElement>("[role=tab]")?.focus();
    for (let count = 0; count < 4; count += 1) {
        const { activeElement } = document;
        focused.push(
            activeElement?.getAttribute("aria-label") ?? activeElement?.textContent ?? null,
        );
        press(activeElement, "Tab");
    }

    expect(focused).toEqual(["a", "a", "c", "c"]);
});

test("activate the pane a person focuses", () => {
    const { container, layout } = renderGroup(PaneLayout.of(["a"], ["b"]));
    container.querySelector<HTMLElement>("input[aria-label=b]")?.focus();
    flush();

    expect(PaneLayout.active(layout())?.items.map(({ id }) => id)).toEqual(["b"]);
});

/** Press a shortcut on an element, as Control on the platform of the tests. */
function shortcut(element: Element | null | undefined, key: string, keys: KeyboardEventInit): void {
    element?.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...keys }),
    );
    flush();
}

/** Write what each pane has open. */
function openIn(layout: PaneLayout): readonly string[] {
    return PaneLayout.panes(layout).map(({ items }) => items.map(({ value }) => value).join(" "));
}

test("split, close, reopen and move by the keys", () => {
    const { container, layout } = renderGroup(PaneLayout.of(["a", "b"]));
    const tab = (name: string) =>
        [...container.querySelectorAll<HTMLElement>("[role=tab]")].find(
            (each) => each.textContent === name,
        );
    const states = [openIn(layout())];
    const act = (name: string, key: string, keys: KeyboardEventInit): void => {
        tab(name)?.focus();
        flush();
        shortcut(tab(name), key, keys);
        states.push(openIn(layout()));
    };
    act("a", "ArrowRight", { shiftKey: true, altKey: true });
    act("a", "w", { ctrlKey: true });
    act("b", "T", { ctrlKey: true, shiftKey: true });
    act("a", "\\", { ctrlKey: true });

    expect(states).toEqual([["a b"], ["b a"], ["b"], ["b a"], ["b a", "a"]]);
});

test("make changes of the same moment one after the other", () => {
    let layout = PaneLayout.of(["a", "b", "c"]);
    const given: PaneLayout[] = [];
    let control: PaneGroupControl | undefined;
    const Probe = (): JSX.Element => {
        control = usePaneGroup();

        return null;
    };
    render(() => (
        <PaneGroup layout={layout} onLayoutChange={(next) => given.push(next)}>
            {() => (
                <Pane>
                    <Probe />
                </Pane>
            )}
        </PaneGroup>
    ));
    control?.close("b");
    control?.close("c");
    layout = given.at(-1) ?? layout;

    expect(openIn(layout)).toEqual(["a"]);
});

test("open a closed pane's tabs again", () => {
    let control: PaneGroupControl | undefined;
    const [layout, setLayout] = createSignal(PaneLayout.of(["a"], ["b", "c"]));
    const Probe = (): JSX.Element => {
        control = usePaneGroup();

        return null;
    };
    render(() => (
        <PaneGroup layout={layout()} onLayoutChange={setLayout}>
            {() => (
                <Pane>
                    <Probe />
                </Pane>
            )}
        </PaneGroup>
    ));
    control?.remove("pane-2");
    flush();
    control?.reopen();
    flush();
    control?.reopen();
    flush();

    expect(openIn(layout())).toEqual(["a c b"]);
});

test("open a tab in a window of its own, and keep it where the browser refuses", () => {
    stubPopovers();
    const opened: unknown[] = [];
    let isRefused = true;
    let control: PaneGroupControl | undefined;
    const [layout, setLayout] = createSignal(PaneLayout.of(["a", "b"]));
    const Probe = (): JSX.Element => {
        control = usePaneGroup();

        return null;
    };
    const { container } = render(() => (
        <PaneGroup
            layout={layout()}
            onLayoutChange={setLayout}
            href={(item) => `/files/${item.value}`}
        >
            {() => (
                <Pane>
                    <Probe />
                    <PaneTabs />
                </Pane>
            )}
        </PaneGroup>
    ));
    const view = container.ownerDocument.defaultView;
    if (view !== null) {
        view.open = (href, target) => {
            opened.push([href, target]);

            return isRefused ? null : view;
        };
    }
    const move = { id: "b", type: "tab", isCopied: false, items: [] };
    const states = [openIn(layout())];
    for (const refused of [true, false]) {
        isRefused = refused;
        control?.move({ ...move, place: { kind: "window" } });
        flush();
        states.push(openIn(layout()));
    }

    expect({
        opened,
        states,
        links: [...container.querySelectorAll("[role=tab]")].map((tab) => tab.getAttribute("href")),
    }).toEqual({
        opened: [
            ["/files/b", "_blank"],
            ["/files/b", "_blank"],
        ],
        states: [["a b"], ["a b"], ["a"]],
        links: ["/files/a"],
    });
});

/** Lay every element out as wide as a phone, or as a desktop, until the test ends. */
function layOut(width: number): void {
    const page = Object.getOwnPropertyDescriptors(HTMLElement.prototype);
    for (const [name, value] of [
        ["clientWidth", width],
        ["clientHeight", 700],
    ] as const) {
        Object.defineProperty(HTMLElement.prototype, name, {
            configurable: true,
            get: () => value,
        });
    }
    onTestFinished(() => {
        // give it back what it had of its own
        for (const name of ["clientWidth", "clientHeight"] as const) {
            const own = page[name];
            if (own === undefined) {
                Reflect.deleteProperty(HTMLElement.prototype, name);
            } else {
                Object.defineProperty(HTMLElement.prototype, name, own);
            }
        }
    });
}

test("show one pane with every tab where two do not fit, and go to the pane of the tab", () => {
    layOut(360);
    const { container, layout } = renderGroup(PaneLayout.of(["a"], ["b", "c"]));
    const shown = () =>
        [...container.querySelectorAll("[data-slot=pane]:not([hidden])")].map((pane) =>
            [...pane.querySelectorAll("[role=tab]")].map((tab) => tab.textContent).join(" "),
        );
    const states = [shown()];
    [...container.querySelectorAll<HTMLElement>("[role=tab]")]
        .find((each) => each.textContent === "c")
        ?.click();
    flush();
    states.push(shown());

    expect({
        states,
        selected: PaneLayout.panes(layout()).map((pane) => PaneLayout.selected(pane)?.id),
        active: layout().active,
    }).toEqual({ states: [["a b c"], ["a b c"]], selected: ["a", "c"], active: "pane-2" });
});

test("keep many panes side by side where two fit", () => {
    layOut(1000);
    const { container } = renderGroup(PaneLayout.of(["a"], ["b"], ["c"], ["d"], ["e"], ["f"]));

    expect(container.querySelectorAll("[data-slot=pane]:not([hidden])").length).toBe(6);
});

test("lock a layout nothing handles a change of", () => {
    stubPopovers();
    const { container } = render(() => <PaneGroup layout={PaneLayout.of(["a", "b"], ["c"])} />);
    flush();

    expect({
        tab: container
            .querySelector("[data-slot=tabs-item], [data-tab]")
            ?.hasAttribute("data-disabled"),
        line: container.querySelector("[data-slot=pane-handle]")?.getAttribute("tabindex"),
    }).toEqual({ tab: true, line: "-1" });
});

test("tell items apart whose ids differ by a space", () => {
    const { container } = renderGroup(PaneLayout.of(["my file", "my-file"]));
    const ids = [...container.querySelectorAll("[role=tab]")].map(({ id }) => id);

    expect(new Set(ids).size).toBe(2);
});

test("share one menu among a pane's tabs", () => {
    const { container } = renderGroup(PaneLayout.of(["a", "b", "c"]));

    expect(container.querySelectorAll("[data-slot=context-menu-content]").length).toBe(1);
});

test("refuse parts of a pane outside one", () => {
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <PaneContent />
        </Errored>
    ));

    expect(container.textContent).toBe("TypeError: a pane's elements need a pane around them");
});
