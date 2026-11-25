import * as style from "@destack/style";
import { expect, test } from "@destack/test";
import { createMemo, createSignal, flush, type JSX } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    type ElementPartProperties,
    mergeProperties,
    type PartAttributes,
    type Render,
    renderPart,
    useRender,
} from "./index.ts";

/** The properties of the parts of these tests. */
type PartProperties = ElementPartProperties &
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style">;

/** The properties of a test part: attributes any element takes, and options only the part reads. */
const PROPERTIES = {
    id: "save",
    "aria-label": "Save note",
    "data-testid": "save",
    variant: "outline",
    disabled: true,
    children: "Save",
};

/** The styles of two test parts, one rendering the other: both set the position. */
const styles = style.create({
    outer: { position: "relative", opacity: 0.5 },
    inner: { position: "sticky", color: "red" },
    caller: { position: "sticky" },
});

/** The attributes a test part gives its element. */
const PART: PartAttributes = { "data-slot": "button", role: "link", tabindex: 0 };

/** Render a plain part. */
function Plain(properties: PartProperties): JSX.Element {
    return renderPart("div", "plain", properties, null, { role: "note" });
}

/** Render a plain part drawn by another. */
function Nested(properties: PartProperties): JSX.Element {
    return <Plain {...properties} render={(part) => <Plain {...part} />} />;
}

test("render a part's own element with every attribute, or the caller's element with the forwarded ones", () => {
    const { container } = render(() => (
        <>
            {useRender({ render: undefined, defaultTagName: "button", part: PART, properties: {} })}
            {useRender({
                render: (attributes) => <a href="#notes" {...attributes} />,
                defaultTagName: "button",
                part: PART,
                properties: PROPERTIES,
            })}
        </>
    ));

    // leave the part's own options, such as its variant, off the caller's element
    expect(markup(container)).toBe(
        '<button data-slot="button" role="link" tabindex="0"></button>' +
            '<a href="#notes" data-slot="button" role="link" tabindex="0" id="save" aria-label="Save note" data-testid="save">Save</a>',
    );
});

test("run the caller's and the part's handlers on the part's own element", () => {
    const calls: string[] = [];
    const { container } = render(() =>
        useRender({
            render: undefined,
            defaultTagName: "button",
            part: { onClick: () => calls.push("part") },
            properties: { onClick: () => calls.push("caller") },
        }),
    );
    container.querySelector("button")?.click();

    expect(calls).toEqual(["caller", "part"]);
});

test("render an element part on its own tag or on the element its caller renders, with its slot and the caller's attributes", () => {
    const { container } = render(() => (
        <>
            {renderPart("div", "card-header", { id: "header", title: "Header" }, null)}
            {renderPart("div", "card-header", { render: (part) => <section {...part} /> }, null, {
                id: "own",
            })}
        </>
    ));

    expect(markup(container)).toBe(
        '<div data-slot="card-header" id="header" title="Header"></div>' +
            '<section data-slot="card-header" id="own"></section>',
    );
});

test("merge a part's attributes under its caller's, running both sides' handlers, the caller's first", () => {
    const calls: string[] = [];
    const merged = mergeProperties(
        { id: "own", title: "Own", onClick: () => calls.push("part") },
        { id: "theirs", title: undefined, onClick: () => calls.push("caller") },
    );
    merged.onClick();

    // the caller's id and undefined title win, and both handlers run
    expect({ id: merged.id, title: merged.title, calls }).toEqual({
        id: "theirs",
        title: undefined,
        calls: ["caller", "part"],
    });
});

test("compose an outer part's styles between a kit part's styles and its caller's, in one call", () => {
    const { container } = render(() =>
        renderPart(
            "div",
            "outer",
            {
                render: (attributes) =>
                    renderPart(
                        "span",
                        "inner",
                        { ...attributes, xstyle: styles.caller },
                        styles.inner,
                    ),
            },
            styles.outer,
        ),
    );

    // resolve the position once, the caller's sticky over the outer part's relative
    expect(container.querySelector("span")?.getAttribute("class")).toBe(
        style.attributes([styles.inner, styles.outer, styles.caller]).class,
    );
});

test("run the handlers of an outer part on the element an inner part's caller renders", () => {
    const ran: string[] = [];
    const { container } = render(() =>
        renderPart(
            "div",
            "outer",
            {
                render: (attributes) =>
                    renderPart(
                        "div",
                        "inner",
                        { ...attributes, render: (inner) => <li {...inner} /> },
                        null,
                    ),
            },
            null,
            {
                onPointerDown: () => void ran.push("down"),
                onContextMenu: () => void ran.push("menu"),
            },
        ),
    );
    const item = container.querySelector("li");
    item?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    item?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));

    expect(ran).toEqual(["down", "menu"]);
});

test("run every reference of lists", () => {
    const ran: string[] = [];
    const mark = (name: string) => () => void ran.push(name);
    render(() => (
        <div
            {...mergeProperties(
                { ref: [mark("part one"), [mark("part two")]] },
                { ref: [mark("caller")] },
            )}
        />
    ));

    expect(ran).toEqual(["caller", "part one", "part two"]);
});

test("run a caller's reference on the element it renders", () => {
    const elements: Element[] = [];
    const { container } = render(() =>
        renderPart(
            "button",
            "button",
            {
                ref: (element: Element) => void elements.push(element),
                render: (attributes) => <a href="#notes" {...attributes} />,
            },
            [],
        ),
    );

    expect(elements).toEqual([container.querySelector("a")]);
});

test("keep a part's content as its styles change", () => {
    const [isSticky, setSticky] = createSignal(false, { ownedWrite: true });
    const { container } = render(() =>
        renderPart("div", "card", {}, () => [isSticky() && styles.caller], {
            get children() {
                return <b>Note</b>;
            },
        }),
    );
    const content = container.querySelector("b");
    setSticky(true);
    flush();

    expect([
        container.querySelector("b") === content,
        container.firstElementChild?.className,
    ]).toEqual([true, style.attributes([styles.caller]).class]);
});

test("build a part's content once, whatever writes what its styles read", () => {
    // draw a part by others, its style reading what changes before or while it builds
    const draws: Record<string, Render> = {
        native: (part) => <i {...part} />,
        part: (part) => <Plain {...part} />,
        nested: (part) => <Nested {...part} />,
    };
    const counts = Object.entries(draws).flatMap(([name, draw]) =>
        ["before", "inside"].map((place) => {
            // read through a memo, stale as the part first reads it
            const [version, setVersion] = createSignal(0, { ownedWrite: true });
            const write = (): void => void setVersion((each) => each + 1);
            let built = 0;
            const Outer = (properties: PartProperties): JSX.Element => {
                const isOdd = createMemo(() => version() % 2 === 1);
                if (place === "before") {
                    write();
                }

                return renderPart("div", "outer", properties, () => [isOdd() && styles.caller]);
            };
            const Content = (): JSX.Element => {
                built += 1;
                if (place === "inside" && built === 1) {
                    write();
                }

                return null;
            };
            render(() => (
                <Outer render={draw}>
                    <Content />
                </Outer>
            ));
            flush();
            write();
            flush();

            return [name, place, built];
        }),
    );

    expect(counts).toEqual([
        ["native", "before", 1],
        ["native", "inside", 1],
        ["part", "before", 1],
        ["part", "inside", 1],
        ["nested", "before", 1],
        ["nested", "inside", 1],
    ]);
});
