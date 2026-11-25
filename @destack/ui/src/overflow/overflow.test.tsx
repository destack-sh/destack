import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, flush, For, type JSX } from "@destack/view";
import { frame, render, stubPopovers } from "@destack/view/test";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../breadcrumb/index.ts";
import { Tabs, TabsList, TabsTrigger } from "../tabs/index.ts";
import { Toolbar, ToolbarButton, ToolbarSeparator } from "../toolbar/index.ts";
import {
    Overflow,
    type OverflowControl,
    OverflowItem,
    OverflowMenu,
    OverflowMenuContent,
    OverflowMenuTrigger,
    useOverflow,
} from "./index.ts";

/** What takes one step of room in a test. */
const STEPS =
    "[data-slot=overflow-item], [data-slot=overflow-menu-trigger], [data-slot=breadcrumb-item], [data-slot=breadcrumb-separator], [data-slot=toolbar-button], [data-slot=toolbar-separator], [role=tab]";

/** Find the row around an element. */
function rowOf(element: Element): Element {
    return element.closest("[data-slot=overflow]") ?? element;
}

/** Give every row room for some steps, each shown one taking one. */
function measureSteps(room: number): void {
    // lay the shown steps of a row out one after the other
    stubPopovers();
    const sizes = {
        clientWidth: () => room * 10,
        offsetWidth: (element: Element) => (shownSteps(rowOf(element)).includes(element) ? 10 : 0),
        offsetLeft: (element: Element) =>
            Math.max(0, shownSteps(rowOf(element)).indexOf(element)) * 10,
        offsetParent: (element: Element) => rowOf(element).firstElementChild,
    };
    const names = Object.entries(sizes);
    const before = names.map(([name]) =>
        Object.getOwnPropertyDescriptor(HTMLElement.prototype, name),
    );
    for (const [name, read] of names) {
        Object.defineProperty(HTMLElement.prototype, name, {
            configurable: true,
            get(this: Element) {
                return read(this);
            },
        });
    }
    onTestFinished(() => {
        names.forEach(([name], index) => {
            const descriptor = before[index];
            if (descriptor !== undefined) {
                Object.defineProperty(HTMLElement.prototype, name, descriptor);
            }
        });
    });
}

/** List the steps shown in an element, a menu in an item as one with it. */
function shownSteps(element: Element): Element[] {
    return [...element.querySelectorAll(STEPS)].filter(
        (step) =>
            step.closest("[hidden]") === null &&
            step.parentElement?.closest("[data-slot=breadcrumb-item]") == null,
    );
}

/** Write what shows, separators as lines and the menu as dots. */
function shownIn(container: Element): string {
    return shownSteps(container)
        .map((step) => {
            const slot = step.getAttribute("data-slot") ?? "";
            const isMenu = step.querySelector("[data-slot=overflow-menu-trigger]") !== null;

            return slot.endsWith("separator")
                ? "|"
                : isMenu || slot === "overflow-menu-trigger"
                  ? "…"
                  : (step.textContent?.trim() ?? "");
        })
        .join(" ");
}

/** Render letters in an overflow. */
function Letters(properties: {
    readonly letters: readonly string[];
    readonly priority?: (letter: string) => number;
    readonly children?: JSX.Element;
}): JSX.Element {
    return (
        <Overflow>
            <For each={properties.letters}>
                {(letter) => (
                    <OverflowItem priority={properties.priority?.(letter) ?? 0}>
                        {letter}
                    </OverflowItem>
                )}
            </For>
            {"children" in properties ? properties.children : <OverflowMenu />}
        </Overflow>
    );
}

/** Read the overflow around it. */
function Probe(properties: { readonly read: (control: OverflowControl) => void }): JSX.Element {
    properties.read(useOverflow());

    return null;
}

test("show what fits before the menu, all of it without one", () => {
    const letters = ["a", "b", "c", "d"];
    const shown = [2, 3, 4].map((room) => {
        measureSteps(room);

        return shownIn(render(() => <Letters letters={letters} />).container);
    });

    expect(shown).toEqual(["a …", "a b …", "a b c d"]);
});

test("hide the lowest priority first, and one of infinity never", () => {
    measureSteps(3);
    const priorities = new Map([
        ["a", -1],
        ["d", Infinity],
        ["e", 1],
    ]);
    const { container } = render(() => (
        <Letters
            letters={["a", "b", "c", "d", "e"]}
            priority={(letter) => priorities.get(letter) ?? 0}
        />
    ));

    expect(shownIn(container)).toBe("d e …");
});

test("hide onward from the menu, then back from it", () => {
    const shown = [4, 3, 2].map((room) => {
        measureSteps(room);

        return shownIn(
            render(() => (
                <Overflow>
                    <OverflowItem>a</OverflowItem>
                    <OverflowMenu />
                    <OverflowItem>b</OverflowItem>
                    <OverflowItem>c</OverflowItem>
                    <OverflowItem>d</OverflowItem>
                    <OverflowItem priority={Infinity}>z</OverflowItem>
                </Overflow>
            )).container,
        );
    });

    expect(shown).toEqual(["a … d z", "a … z", "… z"]);
});

test("follow what it has, showing what fits again", () => {
    measureSteps(3);
    const [letters, setLetters] = createSignal<readonly string[]>(["a", "b", "c"], {
        ownedWrite: true,
    });
    const { container } = render(() => <Letters letters={letters()} />);
    const shown = [shownIn(container)];
    for (const next of [
        ["a", "b", "c", "d"],
        ["b", "c", "d"],
    ]) {
        setLetters(next);
        flush();
        shown.push(shownIn(container));
    }

    expect(shown).toEqual(["a b c", "a b …", "b c d"]);
});

test("show a separator between two that show", () => {
    measureSteps(5);
    const trail = render(() => (
        <Breadcrumb>
            <Overflow>
                <BreadcrumbList>
                    <BreadcrumbItem>Home</BreadcrumbItem>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>
                        <OverflowMenu />
                    </BreadcrumbItem>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>Trips</BreadcrumbItem>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>Lisbon</BreadcrumbItem>
                    <BreadcrumbSeparator />
                    <BreadcrumbItem>
                        <BreadcrumbPage>Day</BreadcrumbPage>
                    </BreadcrumbItem>
                </BreadcrumbList>
            </Overflow>
        </Breadcrumb>
    ));
    const toolbar = render(() => (
        <Toolbar>
            <Overflow>
                <ToolbarButton>B</ToolbarButton>
                <ToolbarButton>I</ToolbarButton>
                <ToolbarButton>U</ToolbarButton>
                <ToolbarSeparator />
                <ToolbarButton>L</ToolbarButton>
                <ToolbarButton>N</ToolbarButton>
                <OverflowMenu>
                    <OverflowMenuTrigger />
                </OverflowMenu>
            </Overflow>
        </Toolbar>
    ));

    expect([shownIn(trail.container), shownIn(toolbar.container)]).toEqual([
        "Home | … | Day",
        "B I U | …",
    ]);
});

test("show a tab chosen from the menu, with the focus and the selection", async () => {
    measureSteps(3);
    const { container } = render(() => (
        <Tabs defaultValue="a">
            <Overflow>
                <TabsList>
                    <For each={["a", "b", "c", "d"]}>
                        {(value) => <TabsTrigger value={value}>{value}</TabsTrigger>}
                    </For>
                </TabsList>
                <OverflowMenu />
            </Overflow>
        </Tabs>
    ));
    const before = shownIn(container);
    const items = [...container.querySelectorAll<HTMLElement>("[role=menuitem]")];
    items.find((item) => item.textContent === "d")?.click();
    flush();
    await frame();
    await frame();
    flush();

    expect({
        before,
        items: items.map((item) => item.textContent),
        after: shownIn(container),
        focused: document.activeElement?.textContent,
    }).toEqual({ before: "a b …", items: ["c", "d"], after: "a d …", focused: "d" });
});

test("tell which do not fit, without a menu", () => {
    measureSteps(2);
    let control: OverflowControl | undefined;
    const { container } = render(() => (
        <Letters letters={["a", "b", "c"]}>
            <Probe read={(each) => (control = each)} />
        </Letters>
    ));
    flush();

    expect({
        shown: shownIn(container),
        hidden: control?.hidden().map((entry) => entry.label()),
    }).toEqual({ shown: "a b", hidden: ["c"] });
});

test("render a menu of one's own", () => {
    measureSteps(2);
    const { container } = render(() => (
        <Letters letters={["a", "b", "c"]}>
            <OverflowMenu>
                <OverflowMenuTrigger>all</OverflowMenuTrigger>
                <OverflowMenuContent>{(entry) => <b>{entry.label()}</b>}</OverflowMenuContent>
            </OverflowMenu>
        </Letters>
    ));
    flush();

    expect({
        trigger: container.querySelector("[data-slot=overflow-menu-trigger]")?.textContent,
        content: container.querySelector("[data-slot=overflow-menu-content]")?.textContent,
    }).toEqual({ trigger: "all", content: "bc" });
});
