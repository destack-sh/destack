import { expect, test } from "@destack/test";
import { createSignal, flush, For } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerProvider,
    MessageScrollerViewport,
    useMessageScroller,
} from "./index.ts";

/** The height of the view, in pixels. */
const VIEW = 200;

/** The height of a row, in pixels. */
const ROW = 50;

/** Lay out a conversation's rows below each other, and follow how far its view scrolls. */
function layOut(container: Element): {
    readonly top: () => number;
    readonly scroll: (top: number) => void;
    readonly change: () => Promise<void>;
} {
    // measure the view, the content and the rows
    const find = (slot: string): HTMLElement => {
        const element = container.querySelector<HTMLElement>(`[data-slot=${slot}]`);
        if (element === null) {
            throw new TypeError(`the test conversation has no '${slot}'`);
        }

        return element;
    };
    const viewport = find("message-scroller-viewport");
    const content = find("message-scroller-content");
    const spacer = find("message-scroller-spacer");
    let top = 0;
    const tall = (): number => content.children.length * ROW;
    const whole = (): number => tall() + (Number.parseFloat(spacer.style.height) || 0);
    const move = (next: number): void => {
        top = Math.max(0, next);
        viewport.dispatchEvent(new Event("scroll"));
        flush();
    };
    const define = (element: Element, values: Readonly<Record<string, () => number>>): void => {
        for (const [name, get] of Object.entries(values)) {
            Object.defineProperty(element, name, { get, set: move, configurable: true });
        }
    };
    define(viewport, {
        scrollTop: () => top,
        clientHeight: () => VIEW,
        scrollHeight: () => Math.max(VIEW, whole()),
    });
    define(content, { offsetHeight: tall });
    content.getBoundingClientRect = () => new DOMRect(0, -top, 0, tall());
    viewport.scrollTo = (options?: ScrollToOptions | number): void =>
        move(typeof options === "object" ? (options.top ?? top) : top);
    const measure = (): void => {
        [...content.children].forEach((row, index) => {
            define(row, { offsetHeight: () => ROW });
            row.getBoundingClientRect = () => new DOMRect(0, index * ROW - top, 0, ROW);
        });
    };
    measure();

    return {
        top: () => top,
        scroll: move,
        change: async () => {
            flush();
            measure();
            await Promise.resolve();
            flush();
        },
    };
}

/** List the ids of some rows. */
function rows(count: number): readonly string[] {
    return Array.from({ length: count }, (_, index) => `row-${String(index)}`);
}

test("render a conversation as a log in a region, with a button out of reach at its end", () => {
    const { container } = render(() => (
        <MessageScroller defaultScrollPosition="start">
            <MessageScrollerViewport>
                <MessageScrollerContent aria-busy="true">
                    <MessageScrollerItem messageId="a" scrollAnchor>
                        Hello
                    </MessageScrollerItem>
                </MessageScrollerContent>
            </MessageScrollerViewport>
            <MessageScrollerButton />
        </MessageScroller>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="message-scroller">',
            '<div data-slot="message-scroller-viewport" role="region" tabindex="0" aria-label="Messages">',
            '<div data-slot="message-scroller-content" role="log" aria-relevant="additions" aria-busy="true">',
            '<div data-slot="message-scroller-item" tabindex="0" data-message-id="a" data-scroll-anchor="">Hello</div></div>',
            '<div aria-hidden="true" data-slot="message-scroller-spacer"></div></div>',
            '<button data-slot="message-scroller-button" data-variant="outline" data-size="icon-sm" type="button" data-direction="end" data-active="false" aria-label="Jump to latest message" inert="" tabindex="-1">',
            '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg>',
            "</button></div>",
        ].join(""),
    );
});

test("go to each end by its button, out of reach there", async () => {
    const { container } = render(() => (
        <MessageScroller defaultScrollPosition="start">
            <MessageScrollerViewport>
                <MessageScrollerContent>
                    <For each={rows(10)}>{(id) => <MessageScrollerItem messageId={id} />}</For>
                </MessageScrollerContent>
            </MessageScrollerViewport>
            <MessageScrollerButton direction="start" behavior="instant" />
            <MessageScrollerButton behavior="instant" />
        </MessageScroller>
    ));
    const layout = layOut(container);
    await layout.change();
    layout.scroll(0);
    const buttons = [...container.querySelectorAll<HTMLElement>("button")];
    const read = (): readonly unknown[] => [
        layout.top(),
        ...buttons.map((button) => button.getAttribute("data-active")),
    ];
    const path = [read()];
    for (const button of buttons.toReversed()) {
        button.click();
        flush();
        path.push(read());
    }

    expect(path).toEqual([
        [0, "false", "true"],
        [300, "true", "false"],
        [0, "false", "true"],
    ]);
});

test("bring a new turn near the top, keep the place of earlier rows, and count those unseen", async () => {
    // open at the start of ten rows
    const [ids, setIds] = createSignal(rows(10));
    let control: ReturnType<typeof useMessageScroller> | undefined;
    const Probe = (): null => {
        control = useMessageScroller();

        return null;
    };
    const { container } = render(() => (
        <MessageScrollerProvider autoScroll defaultScrollPosition="start">
            <Probe />
            <MessageScroller>
                <MessageScrollerViewport>
                    <MessageScrollerContent>
                        <For each={ids()}>
                            {(id) => (
                                <MessageScrollerItem messageId={id} scrollAnchor={id === "turn"} />
                            )}
                        </For>
                    </MessageScrollerContent>
                </MessageScrollerViewport>
            </MessageScroller>
        </MessageScrollerProvider>
    ));
    const layout = layOut(container);
    await layout.change();
    const read = (): readonly unknown[] => [
        layout.top(),
        control?.unseen(),
        container.querySelector("[data-slot=message-scroller]")?.getAttribute("data-scrollable"),
    ];

    // add a row below while the reader is at the fourth, and one above
    layout.scroll(160);
    const start = read();
    setIds((all) => [...all, "below"]);
    await layout.change();
    const below = read();
    setIds((all) => ["above", ...all]);
    await layout.change();
    const above = read();

    // follow from the end, then start a turn
    control?.scrollToEnd();
    await layout.change();
    const end = read();
    setIds((all) => [...all, "another"]);
    await layout.change();
    const followed = read();
    setIds((all) => [...all, "turn"]);
    await layout.change();

    expect({
        start,
        below,
        above,
        end,
        followed,
        turn: read(),
        jump: [control?.scrollToMessage("row-2"), layout.top(), control?.scrollToMessage("none")],
    }).toEqual({
        start: [160, 0, "start end"],
        below: [160, 1, "start end"],
        above: [210, 1, "start end"],
        end: [400, 0, "start"],
        followed: [450, 0, "start"],
        turn: [586, 0, "start"],
        jump: [true, 150, false],
    });
});
