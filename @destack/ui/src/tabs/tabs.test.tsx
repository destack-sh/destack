import { expect, test } from "@destack/test";
import { createSignal, Errored, flush, For, type JSX } from "@destack/view";
import { markup, render } from "@destack/view/test";
import { tabsHistoryUnavailable } from "./tabs.example.tsx";
import { Tabs, TabsClose, TabsIndicator, TabsItem, TabsList, TabsTrigger } from "./tabs.tsx";
import { type TabsControl, useTabs } from "./index.ts";

test("connect each tab to its panel and show only the selected panel", () => {
    const frame = render(tabsHistoryUnavailable).container.querySelector("[data-slot=example]");
    expect(frame === null ? "" : markup(frame)).toBe(
        '<div data-slot="tabs" data-orientation="horizontal">' +
            '<div data-slot="tabs-list" role="tablist" data-variant="default" data-orientation="horizontal" aria-orientation="horizontal" aria-label="Note"><div role="presentation">' +
            '<button data-slot="tabs-trigger" type="button" role="tab" id="id-1-tab-edit" aria-controls="id-1-panel-edit" aria-selected="true" tabindex="0" data-active="" data-value="edit" data-orientation="horizontal">Edit</button>' +
            '<button data-slot="tabs-trigger" type="button" role="tab" id="id-1-tab-preview" aria-controls="id-1-panel-preview" aria-selected="false" tabindex="-1" data-value="preview" data-orientation="horizontal">Preview</button>' +
            '<button data-slot="tabs-trigger" type="button" role="tab" id="id-1-tab-history" aria-controls="id-1-panel-history" aria-selected="false" tabindex="-1" data-value="history" data-orientation="horizontal" disabled="">History</button></div></div>' +
            '<div role="tabpanel" id="id-1-panel-edit" aria-labelledby="id-1-tab-edit" tabindex="0" data-slot="tabs-content" data-orientation="horizontal">Editor</div>' +
            '<div role="tabpanel" id="id-1-panel-preview" aria-labelledby="id-1-tab-preview" tabindex="0" hidden="" data-slot="tabs-content" data-hidden="" data-orientation="horizontal">Rendered</div>' +
            '<div role="tabpanel" id="id-1-panel-history" aria-labelledby="id-1-tab-history" tabindex="0" hidden="" data-slot="tabs-content" data-hidden="" data-orientation="horizontal">Versions</div></div>',
    );
});

/** Render tabs that close, the first pinned by having no close button. */
function Closable(properties: { readonly onValueChange?: (value: string) => void }): JSX.Element {
    const [tabs, setTabs] = createSignal<readonly string[]>(["a", "b", "c", "d"], {
        ownedWrite: true,
    });

    return (
        <Tabs
            defaultValue="b"
            onValueChange={(value) => properties.onValueChange?.(value)}
            onClose={(closed) => setTabs((all) => all.filter((tab) => tab !== closed))}
        >
            <TabsList>
                <For each={tabs()}>
                    {(tab) => (
                        <TabsItem value={tab}>
                            <TabsTrigger>{tab}</TabsTrigger>
                            {tab === "a" ? undefined : <TabsClose />}
                        </TabsItem>
                    )}
                </For>
            </TabsList>
        </Tabs>
    );
}

/** Write the tabs, the selected one in brackets. */
function tabsIn(container: Element): string {
    return [...container.querySelectorAll("[role=tab]")]
        .map((tab) =>
            tab.getAttribute("aria-selected") === "true"
                ? `[${tab.textContent ?? ""}]`
                : tab.textContent,
        )
        .join(" ");
}

/** Press and release the middle button. */
function middleClick(): MouseEvent {
    return new MouseEvent("auxclick", { button: 1, bubbles: true });
}

/** Read the classes of the first tab. */
function classOf(root: Element): string | undefined {
    return root.querySelector("[role=tab]")?.className;
}

test("close a tab by its button, by the middle button and by Delete, unless it has no button", () => {
    const { container } = render(() => <Closable />);
    const tab = (name: string) =>
        [...container.querySelectorAll<HTMLElement>("[role=tab]")].find(
            (each) => each.textContent === name,
        );
    const states = [tabsIn(container)];
    const act = (run: () => void): void => {
        run();
        flush();
        states.push(tabsIn(container));
    };
    act(() => tab("d")?.dispatchEvent(middleClick()));
    act(() => tab("a")?.dispatchEvent(middleClick()));
    act(() => {
        tab("b")?.focus();
        tab("b")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true }));
    });
    act(() => container.querySelector<HTMLElement>("[data-slot=tabs-close]")?.click());

    expect(states).toEqual(["a [b] c d", "a [b] c", "a [b] c", "a [c]", "[a]"]);
});

test("select the tab after a removed one, telling the owner once", () => {
    const told: string[] = [];
    const { container } = render(() => <Closable onValueChange={(value) => told.push(value)} />);
    container.querySelectorAll<HTMLElement>("[data-slot=tabs-close]")[0]?.click();
    flush();

    expect({ tabs: tabsIn(container), told }).toEqual({ tabs: "a [c] d", told: ["c"] });
});

test("mark the selected tab by an indicator in place of its own mark", () => {
    const { container } = render(() => (
        <Tabs defaultValue="a">
            <TabsList>
                <TabsTrigger value="a">a</TabsTrigger>
                <TabsIndicator />
            </TabsList>
        </Tabs>
    ));
    flush();
    const plain = render(() => (
        <Tabs defaultValue="a">
            <TabsList>
                <TabsTrigger value="a">a</TabsTrigger>
            </TabsList>
        </Tabs>
    ));

    expect({
        isShown: container.querySelector("[data-slot=tabs-indicator]")?.hasAttribute("hidden"),
        isSame: classOf(container) === classOf(plain.container),
    }).toEqual({ isShown: false, isSame: false });
});

/** Read the error a part is refused with. */
function refused(view: () => JSX.Element): string {
    return render(() => (
        <Errored fallback={(error) => String(error())}>
            <Tabs>{view()}</Tabs>
        </Errored>
    )).container.textContent;
}

test("refuse a tab without a value, and a close button outside an item", () => {
    expect([refused(() => <TabsTrigger>a</TabsTrigger>), refused(() => <TabsClose />)]).toEqual([
        "TypeError: a tab is invalid: it has neither a value nor an item around it",
        "TypeError: a tab's close button needs a tab's item around it",
    ]);
});

test("close other tabs, and those after, and step among them, by the control a menu uses", () => {
    let control: TabsControl | undefined;
    const Probe = (): JSX.Element => {
        control = useTabs();

        return null;
    };
    const closed: string[] = [];
    render(() => (
        <Tabs defaultValue="b" onClose={(value) => closed.push(value)} onMove={() => undefined}>
            <Probe />
            <TabsList>
                <For each={["a", "b", "c", "d"]}>
                    {(tab) => (
                        <TabsItem value={tab}>
                            <TabsTrigger>{tab}</TabsTrigger>
                            {tab === "a" ? undefined : <TabsClose />}
                        </TabsItem>
                    )}
                </For>
            </TabsList>
        </Tabs>
    ));
    flush();
    const closable = (["all", "others", "after"] as const).map((place) =>
        control?.closable(place, "b"),
    );
    control?.step(-2);
    flush();

    expect({
        closable,
        stepped: control?.value(),
        moves: [control?.canMove("a", -1), control?.canMove("a", 1), control?.canMove("d", 1)],
        closed,
    }).toEqual({
        closable: [
            ["b", "c", "d"],
            ["c", "d"],
            ["c", "d"],
        ],
        stepped: "d",
        moves: [false, true, false],
        closed: [],
    });
});
