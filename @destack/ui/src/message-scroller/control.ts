import {
    type Accessor,
    createContext,
    createSignal,
    flush,
    untrack,
    useContext,
} from "@destack/view";

/** The distance from an end within which a reader is at it, in pixels, as in shadcn/ui. */
const EDGE = 8;

/** The room left of what came before above a new turn, in pixels, as in shadcn/ui. */
const PEEK = 64;

/** The rows in view that are looked through for one to hold, past any markers between messages. */
const HELD = 8;

/** The attribute of a row a reader jumps to. */
export const MESSAGE_ID = "data-message-id";

/** The attribute of a row that starts a turn. */
export const SCROLL_ANCHOR = "data-scroll-anchor";

/** The keys that scroll up. */
const UPWARD: ReadonlySet<string> = new Set(["ArrowUp", "PageUp", "Home"]);

/** Where a conversation opens: at its first message, at its end, or at its last turn. */
export type MessagePosition = "start" | "end" | "last-anchor";

/** Where in the view a row goes. */
export type MessageAlignment = "start" | "center" | "end" | "nearest";

/** The ends a conversation scrolls toward. */
export interface MessageScrollable {
    /** Whether something is above the view. */
    readonly start: boolean;
    /** Whether something is below the view. */
    readonly end: boolean;
}

/** How a conversation scrolls. */
export interface MessageScrollerOptions {
    /** Whether it follows what is written while the reader is at its end, off by default. */
    readonly autoScroll?: boolean;
    /** Where it opens with its first rows, at its end by default. */
    readonly defaultScrollPosition?: MessagePosition;
    /** The distance from an end that counts as being there, in pixels, 8 by default. */
    readonly scrollEdgeThreshold?: number;
    /** The room between a row and the edge it goes to, in pixels, none by default. */
    readonly scrollMargin?: number;
    /** The room left of what came before above a new turn, in pixels, 64 by default. */
    readonly scrollPreviousItemPeek?: number;
}

/** How to scroll to an end. */
export interface MessageScrollOptions {
    /** How it moves, at once by default. */
    readonly behavior?: ScrollBehavior;
}

/** How to scroll to a row. */
export interface MessageJumpOptions extends MessageScrollOptions {
    /** Where in the view the row goes, its start by default. */
    readonly align?: MessageAlignment;
    /** The room between the row and the edge, in pixels, the conversation's by default. */
    readonly scrollMargin?: number;
}

/** A row in view, and the place it keeps. */
interface Held {
    /** The row. */
    readonly row: HTMLElement;
    /** How far the view scrolls to bring its top to its own, in pixels. */
    readonly top: number;
}

/** A row asked for before there were any. */
interface Jump {
    /** The row's id. */
    readonly id: string;
    /** How to scroll to it. */
    readonly options: MessageJumpOptions;
}

/** The elements of a conversation, and the way their rows are read. */
class MessageScrollerLayout {
    /** The element that scrolls. */
    readonly viewport: HTMLElement;
    /** The element around the rows. */
    readonly content: HTMLElement;
    /** The room below the rows that lets a new turn start near the top. */
    readonly spacer: HTMLElement;
    /** The rows, from the first to the last. */
    rows: readonly HTMLElement[];

    /** The row Tab goes to, the last one at first. */
    #stop: HTMLElement | undefined;

    /** Take the elements and read their rows. */
    constructor(viewport: HTMLElement, content: HTMLElement, spacer: HTMLElement) {
        // start without rows
        this.viewport = viewport;
        this.content = content;
        this.spacer = spacer;
        this.rows = [];
        this.read();
    }

    /** Read the rows there are now, and the one Tab goes to. */
    read(): void {
        this.rows = [...this.content.children].filter((row) => row instanceof HTMLElement);
        if (this.#stop === undefined || !this.rows.includes(this.#stop)) {
            this.stopAt(this.rows.findLast((row) => row.hasAttribute(MESSAGE_ID)));
        }
    }

    /** Let Tab go to a row, in place of the one before. */
    stopAt(row: HTMLElement | undefined): void {
        if (row !== this.#stop) {
            this.#stop?.setAttribute("tabindex", "-1");
            row?.setAttribute("tabindex", "0");
            this.#stop = row;
        }
    }

    /** Read how far a row's top is below the top of the view. */
    offsetOf(row: HTMLElement): number {
        const { viewport } = this;
        const top = row.getBoundingClientRect().top - viewport.getBoundingClientRect().top;

        return top - viewport.clientTop;
    }

    /** Read how far the view scrolls to bring a row's top to its own. */
    topOf(row: HTMLElement): number {
        return this.offsetOf(row) + this.viewport.scrollTop;
    }

    /** Read how far the view scrolls to show the last row's end. */
    end(): number {
        const { content, viewport } = this;

        return Math.max(0, this.topOf(content) + content.offsetHeight - viewport.clientHeight);
    }

    /** Find the place of the first row in view, the count of rows where none is. */
    firstInView(): number {
        // halve the rows, as each starts below the one before it
        let low = 0;
        let high = this.rows.length;
        while (low < high) {
            const middle = (low + high) >> 1;
            const row = this.rows[middle];
            const isAbove = row !== undefined && this.offsetOf(row) + row.offsetHeight <= 0;
            low = isAbove ? middle + 1 : low;
            high = isAbove ? high : middle;
        }

        return low;
    }
}

/** Where a reader is in a conversation, and how it moves: never against what they do. */
export class MessageScrollerControl {
    /** The ends the view scrolls toward. */
    readonly scrollable: Accessor<MessageScrollable>;
    /** The element that scrolls, such as for a virtualizer of the rows. */
    readonly viewport: Accessor<HTMLElement | undefined>;
    /** Whether the view is on its way to the end. */
    readonly isAutoScrolling: Accessor<boolean>;
    /** Whether it waits for the rows it opens at. */
    readonly isPending: Accessor<boolean>;
    /** How many rows came below the view since the reader was at the end. */
    readonly unseen: Accessor<number>;
    /** The id of the turn the reader is in, undefined before the first. */
    readonly currentAnchorId: Accessor<string | undefined>;
    /** The ids of the rows in view, in their order. */
    readonly visibleMessageIds: Accessor<readonly string[]>;
    /** Where it opens. */
    readonly position: MessagePosition;
    /** The options. */
    readonly #options: MessageScrollerOptions;
    /** Set what the parts read. */
    readonly #set: {
        readonly scrollable: (scrollable: MessageScrollable) => void;
        readonly viewport: (viewport: HTMLElement | undefined) => void;
        readonly autoScrolling: (isAutoScrolling: boolean) => void;
        readonly pending: (isPending: boolean) => void;
        readonly unseen: (count: number) => void;
        readonly anchor: (id: string | undefined) => void;
        readonly visible: (ids: readonly string[]) => void;
    };
    /** The elements, undefined until they mount. */
    #layout: MessageScrollerLayout | undefined;
    /** Whether the reader is at the end and stays there. */
    #isFollowing = true;
    /** Whether a person has text of the rows selected. */
    #isSelecting = false;
    /** Where the view was told to go, which rows drawn on the way move, undefined at rest. */
    #destination: (() => number) | undefined;
    /** Whether it goes there smoothly. */
    #isSmooth = false;
    /** Whether a part reads what is in view. */
    #isTracked = false;
    /** The turn that stays near the top while its reply grows, undefined without one. */
    #turn: HTMLElement | undefined;
    /** The row whose place the reader keeps. */
    #held: Held | undefined;
    /** The row to jump to once there are rows. */
    #jump: Jump | undefined;
    /** How far the view was scrolled. */
    #top = 0;
    /** How tall the view was. */
    #height = 0;

    /** Follow how a conversation scrolls. */
    constructor(options: MessageScrollerOptions) {
        // start at rest, waiting for rows where it opens elsewhere than at the start
        this.position = untrack(() => options.defaultScrollPosition) ?? "end";
        const owned = { ownedWrite: true };
        const [scrollable, setScrollable] = createSignal<MessageScrollable>(
            { start: false, end: false },
            {
                ...owned,
                equals: (one, other) => one.start === other.start && one.end === other.end,
            },
        );
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, owned);
        const [autoScrolling, setAutoScrolling] = createSignal(false, owned);
        const [pending, setPending] = createSignal(this.position !== "start", owned);
        const [unseen, setUnseen] = createSignal(0, owned);
        const [anchor, setAnchor] = createSignal<string | undefined>(undefined, owned);
        const [visible, setVisible] = createSignal<readonly string[]>([], owned);
        this.#options = options;
        this.scrollable = scrollable;
        this.viewport = viewport;
        this.isAutoScrolling = autoScrolling;
        this.isPending = pending;
        this.unseen = unseen;

        // read what is in view only where a part asks for it
        this.currentAnchorId = () => {
            this.#track();

            return anchor();
        };
        this.visibleMessageIds = () => {
            this.#track();

            return visible();
        };
        this.#set = {
            scrollable: (next) => void setScrollable(next),
            viewport: (next) => void setViewport(next),
            autoScrolling: (next) => void setAutoScrolling(next),
            pending: (next) => void setPending(next),
            unseen: (next) => void setUnseen(next),
            anchor: (next) => void setAnchor(next),
            visible: (next) => void setVisible(next),
        };
    }

    /** Scroll to the end, and report whether it may. */
    scrollToEnd({ behavior = "auto" }: MessageScrollOptions = {}): boolean {
        // leave the turn, and follow from there
        const layout = this.#layout;
        if (layout === undefined) {
            return false;
        }
        this.#turn = undefined;
        this.#isFollowing = true;
        this.#makeRoom(layout);
        this.#move(layout, () => layout.end(), behavior);
        this.#set.autoScrolling(this.#isSmooth);

        return true;
    }

    /** Scroll to the start, and report whether it may. */
    scrollToStart({ behavior = "auto" }: MessageScrollOptions = {}): boolean {
        // let go of the end
        const layout = this.#layout;
        if (layout === undefined) {
            return false;
        }
        this.#isFollowing = false;
        this.#move(layout, () => 0, behavior);

        return true;
    }

    /** Scroll to a row by its id, waiting for the first rows, and report whether it may. */
    scrollToMessage(id: string, options: MessageJumpOptions = {}): boolean {
        // wait until there are rows
        const layout = this.#layout;
        const rows = layout?.rows ?? [];
        const row = rows.find((each) => each.getAttribute(MESSAGE_ID) === id);
        if (layout === undefined || row === undefined) {
            this.#jump = rows.length === 0 ? { id, options } : undefined;

            return this.#jump !== undefined;
        }

        // bring the row to its place in the view, where rows drawn on the way move it
        const { align = "start", behavior = "auto", scrollMargin = this.#margin() } = options;
        const { viewport } = layout;
        const from = viewport.scrollTop;
        this.#isFollowing = false;
        this.#move(
            layout,
            () => {
                // measure the row in the view
                const start = layout.topOf(row) - scrollMargin;
                const after = layout.topOf(row) + row.offsetHeight + scrollMargin;
                const end = after - viewport.clientHeight;
                const nearest = start < from ? start : end > from ? Math.min(start, end) : from;

                return { start, center: (start + end) / 2, end, nearest }[align];
            },
            behavior,
        );

        return true;
    }

    /** Take the elements, and follow the reader and the rows until it returns. */
    observe(viewport: HTMLElement, content: HTMLElement, spacer: HTMLElement): () => void {
        // open before the rows show
        const layout = new MessageScrollerLayout(viewport, content, spacer);
        this.#layout = layout;
        this.#set.viewport(viewport);
        this.#open(layout);

        // follow the reader
        const listening = new AbortController();
        const passive = { signal: listening.signal, passive: true };
        const { ownerDocument } = viewport;
        viewport.addEventListener("scroll", () => this.#scrolled(layout), passive);
        viewport.addEventListener("scrollend", () => this.#arrived(layout), passive);
        viewport.addEventListener("wheel", (event) => this.#wheeled(layout, event), passive);
        viewport.addEventListener("touchstart", () => this.#stop(), passive);
        viewport.addEventListener("pointerdown", () => this.#stop(), passive);
        viewport.addEventListener("keydown", (event) => this.#pressed(layout, event), passive);
        content.addEventListener("focusin", (event) => this.#focused(layout, event), passive);
        ownerDocument.addEventListener("selectionchange", () => this.#selected(layout), passive);

        // follow the rows
        const resizes = new ResizeObserver(() => this.#grown(layout));
        resizes.observe(viewport);
        resizes.observe(content);
        const changes = new MutationObserver(() => this.#changed(layout));
        changes.observe(content, { childList: true });

        return () => {
            // forget the elements
            listening.abort();
            resizes.disconnect();
            changes.disconnect();
            this.#layout = undefined;
            this.#set.viewport(undefined);
        };
    }

    /** Open once there are rows, before they show. */
    #open(layout: MessageScrollerLayout): void {
        // leave the look it waits in, which holds the rows where they open without JavaScript
        this.#height = layout.viewport.clientHeight;
        this.#set.pending(false);
        queueMicrotask(() => {
            // open where it is still there
            flush();
            if (layout === this.#layout && layout.rows.length > 0) {
                this.#openAt(layout);
            }
        });
    }

    /** Open where the options say. */
    #openAt(layout: MessageScrollerLayout): void {
        // open at the last turn where its reply is taller than the view, else at an end
        const { position } = this;
        const turn = layout.rows.findLast((row) => row.hasAttribute(SCROLL_ANCHOR));
        const start = turn === undefined ? 0 : this.#startOf(layout, turn);
        const isTall = turn !== undefined && layout.end() > start;
        const isTurn = position === "last-anchor" && isTall;
        const place = (): number =>
            position === "start" ? 0 : isTurn ? this.#startOf(layout, turn) : layout.end();
        this.#move(layout, place, "instant");
        this.#isFollowing = this.#isEnd(layout);

        // go on to a row asked for meanwhile
        const jump = this.#jump;
        this.#jump = undefined;
        if (jump !== undefined) {
            this.scrollToMessage(jump.id, { ...jump.options, behavior: "instant" });
        }
    }

    /** Follow rows that came or went. */
    #changed(layout: MessageScrollerLayout): void {
        // open with the first rows
        const before = layout.rows;
        layout.read();
        if (before.length === 0) {
            this.#open(layout);

            return;
        }

        // bring a new turn near the top, the rest of the view for its reply
        const last = layout.rows.findLastIndex((row) => before.includes(row));
        const added = layout.rows.slice(last + 1);
        const turn = added.findLast((row) => row.hasAttribute(SCROLL_ANCHOR));
        if (turn === undefined) {
            this.#grown(layout);
        } else {
            this.#turn = turn;
            this.#isFollowing = true;
            this.#makeRoom(layout);
            this.#move(layout, () => this.#startOf(layout, turn), "smooth");
        }

        // count what came below the view
        if (added.length > 0 && turn === undefined && !this.#isEnd(layout)) {
            this.#set.unseen(untrack(this.unseen) + added.length);
        }
    }

    /** Follow rows or a view that grew or shrank: text that streams, or a keyboard that opens. */
    #grown(layout: MessageScrollerLayout): void {
        // stay at the end where the view got smaller, or the text that streams has no room left
        const { viewport } = layout;
        const isResized = viewport.clientHeight !== this.#height;
        this.#height = viewport.clientHeight;
        const room = this.#makeRoom(layout);
        const isFollowed =
            this.#options.autoScroll === true && this.#isFollowing && !this.#isSelecting;
        const isKept = isResized && !untrack(this.scrollable).end;
        if (room === 0 && (isFollowed || isKept)) {
            this.#turn = undefined;
            this.#move(layout, () => layout.end(), this.#isSmooth ? "smooth" : "instant");
        }
        // arrive again where rows drawn since moved the place
        else if (this.#destination !== undefined) {
            if (!this.#isSmooth) {
                this.#move(layout, this.#destination, "instant");
            }
        }
        // keep the place of the first message in view else
        else {
            this.#keep(layout);
        }
        this.#settle(layout);
    }

    /** Follow the reader's scrolling: up lets go, and back at the end follows again. */
    #scrolled(layout: MessageScrollerLayout): void {
        // leave out rows that shrank, which move the view up to the end
        const { viewport } = layout;
        const top = viewport.scrollTop;
        const isOwn = top === this.#top || top >= viewport.scrollHeight - viewport.clientHeight - 1;
        if (!isOwn && !this.#isSmooth) {
            this.#stop();
        }
        const isUp = top < this.#top && top < layout.end() - 1;
        if (isUp && this.#destination === undefined) {
            this.#isFollowing = false;
        } else if (top >= this.#top && this.#isEnd(layout)) {
            this.#isFollowing = true;
        }

        // keep the place of the first message, as rows drawn since the last frame moved it
        const isClamped = top >= viewport.scrollHeight - viewport.clientHeight - 1;
        if (!isClamped) {
            this.#top = top;
        }
        if (this.#destination === undefined) {
            this.#keep(layout);
        }
        this.#top = viewport.scrollTop;

        // arrive where no event says so, as in Safari before 26.2
        const left = Math.abs(this.#target(layout) - top);
        if (this.#isSmooth && left < 1) {
            this.#stop();
        }
        this.#settle(layout);
    }

    /** Keep the place of the first message in view, by how far rows above it grew or shrank. */
    #keep(layout: MessageScrollerLayout): void {
        // move on from where the view was, as the browser may have moved it to a nearer end
        const held = this.#held;
        const grown = held?.row.isConnected === true ? layout.topOf(held.row) - held.top : 0;
        if (held !== undefined && Math.abs(grown) >= 1) {
            this.#held = { row: held.row, top: held.top + grown };
            this.#top += grown;
            layout.viewport.scrollTop = this.#top;
            this.#top = layout.viewport.scrollTop;
        }
    }

    /** Let go where the reader scrolls up by the wheel. */
    #wheeled(layout: MessageScrollerLayout, { deltaY, target }: WheelEvent): void {
        // leave a wheel over something inside that scrolls up itself
        this.#stop();
        let element = target instanceof Element ? target : null;
        while (element !== null && element !== layout.viewport && element.scrollTop === 0) {
            element = element.parentElement;
        }
        if (deltaY < 0 && element === layout.viewport && layout.viewport.scrollTop > 0) {
            this.#isFollowing = false;
        }
    }

    /** Let go where the reader scrolls up by the keys. */
    #pressed(layout: MessageScrollerLayout, { key, target }: KeyboardEvent): void {
        this.#stop();
        if (UPWARD.has(key) && target === layout.viewport && layout.viewport.scrollTop > 0) {
            this.#isFollowing = false;
        }
    }

    /** Let Tab come back to the row the focus was in. */
    #focused(layout: MessageScrollerLayout, { target }: FocusEvent): void {
        const row = layout.rows.find((each) => target instanceof Node && each.contains(target));
        if (row?.hasAttribute(MESSAGE_ID) === true) {
            layout.stopAt(row);
        }
    }

    /** Stay put while the reader selects text, which is hard to do while it moves. */
    #selected({ viewport }: MessageScrollerLayout): void {
        const selection = viewport.ownerDocument.getSelection();
        this.#isSelecting =
            selection?.isCollapsed === false && viewport.contains(selection.anchorNode);
    }

    /** Stop being on the way, as the reader moves the view. */
    #stop(): void {
        this.#destination = undefined;
        this.#isSmooth = false;
        this.#set.autoScrolling(false);
    }

    /** Arrive, at once where rows drawn on the way moved the place. */
    #arrived(layout: MessageScrollerLayout): void {
        if (this.#isSmooth && this.#destination !== undefined) {
            this.#move(layout, this.#destination, "instant");
            this.#set.autoScrolling(false);
        }
    }

    /** Scroll the view to a place, at once where the reader wants less motion. */
    #move(layout: MessageScrollerLayout, place: () => number, behavior: ScrollBehavior): void {
        // know the way from the reader's own, unless it is there already
        const { viewport } = layout;
        const window = viewport.ownerDocument.defaultView;
        const isReduced = window?.matchMedia("(prefers-reduced-motion: reduce)").matches === true;
        this.#destination = place;
        const top = this.#target(layout);
        const way = top - viewport.scrollTop;
        const isSmooth = behavior === "smooth" && !isReduced && Math.abs(way) >= 1;

        // skip all but the last view of a long way
        if (isSmooth && Math.abs(way) > 2 * viewport.clientHeight) {
            const before = top - Math.sign(way) * viewport.clientHeight;
            viewport.scrollTo({ top: before, behavior: "instant" });
        }
        this.#isSmooth = isSmooth;
        viewport.scrollTo({ top, behavior: isSmooth ? "smooth" : "instant" });
        this.#top = viewport.scrollTop;
        this.#settle(layout);

        // rest once the rows there are drawn, after the next frame
        if (!isSmooth) {
            window?.requestAnimationFrame(() =>
                window.requestAnimationFrame(() => {
                    // leave a place it was told to go to since
                    if (this.#destination === place) {
                        this.#stop();
                    }
                }),
            );
        }
    }

    /** Read how far the view scrolls at its destination, where it is without one. */
    #target({ viewport }: MessageScrollerLayout): number {
        const top = this.#destination?.() ?? viewport.scrollTop;

        return Math.min(Math.max(0, top), viewport.scrollHeight - viewport.clientHeight);
    }

    /** Give a turn near the top the room its reply grows in, and read the room. */
    #makeRoom(layout: MessageScrollerLayout): number {
        // fill what the rows from the turn on leave of the view
        const { content, spacer, viewport } = layout;
        const turn = this.#turn?.isConnected === true ? this.#turn : undefined;
        const bottom = layout.topOf(content) + content.offsetHeight;
        const top = turn === undefined ? 0 : this.#startOf(layout, turn);
        const room = turn === undefined ? 0 : Math.max(0, top + viewport.clientHeight - bottom);
        spacer.style.height = `${String(room)}px`;

        return room;
    }

    /** Say where the reader is, and keep their place. */
    #settle(layout: MessageScrollerLayout): void {
        // tell the parts, nothing unseen at the end
        const { viewport } = layout;
        const isEnd = this.#isEnd(layout);
        this.#set.scrollable({ start: viewport.scrollTop > this.#edge(), end: !isEnd });
        if (isEnd) {
            this.#set.unseen(0);
        }

        // hold the first message that starts in view, as one above its edge may grow or shrink
        const place = layout.firstInView();
        const rows = layout.rows.slice(place, place + HELD);
        const messages = rows.filter((each) => each.hasAttribute(MESSAGE_ID));
        const starting = messages.find((each) => {
            const offset = layout.offsetOf(each);

            return offset >= 0 && offset < viewport.clientHeight;
        });
        const row = starting ?? messages[0] ?? rows[0];
        this.#held = row === undefined ? undefined : { row, top: layout.topOf(row) };
        if (this.#isTracked) {
            this.#see(layout, place);
        }
    }

    /** Read what is in view, from the first row in it. */
    #see(layout: MessageScrollerLayout, first: number): void {
        // list the rows up to the first below the view
        const height = layout.viewport.clientHeight;
        const visible: HTMLElement[] = [];
        for (const row of layout.rows.slice(first)) {
            if (layout.offsetOf(row) >= height) {
                break;
            }
            visible.push(row);
        }

        // find the last turn that starts above the line a new one starts at
        const line = this.#line() + 1;
        const reached = visible.findLastIndex((row) => layout.offsetOf(row) <= line);
        const above = layout.rows.slice(0, first + reached + 1);
        const anchor = above.findLast((row) => row.hasAttribute(SCROLL_ANCHOR));
        this.#set.anchor(anchor?.getAttribute(MESSAGE_ID) ?? undefined);
        this.#set.visible(visible.flatMap((row) => row.getAttribute(MESSAGE_ID) ?? []));
    }

    /** Read what is in view from now on. */
    #track(): void {
        // read it once the parts that ask are there
        if (!this.#isTracked) {
            this.#isTracked = true;
            queueMicrotask(() => {
                if (this.#layout !== undefined) {
                    this.#settle(this.#layout);
                }
            });
        }
    }

    /** Report whether the last row's end is in view. */
    #isEnd(layout: MessageScrollerLayout): boolean {
        return layout.viewport.scrollTop >= layout.end() - this.#edge();
    }

    /** Read the distance from an end that counts as being there. */
    #edge(): number {
        return this.#options.scrollEdgeThreshold ?? EDGE;
    }

    /** Read the room between a row and the edge it goes to. */
    #margin(): number {
        return this.#options.scrollMargin ?? 0;
    }

    /** Read how far the view scrolls to start a turn, some of what came before above it. */
    #startOf(layout: MessageScrollerLayout, turn: HTMLElement): number {
        return layout.topOf(turn) - this.#line();
    }

    /** Read how far below the top of the view a turn starts. */
    #line(): number {
        return this.#margin() + (this.#options.scrollPreviousItemPeek ?? PEEK);
    }
}

/** The conversation of the nearest provider, null outside one. */
export const MessageScrollerContext = createContext<MessageScrollerControl | null>(null);

/** Read the nearest conversation, refusing a part outside one. */
export function useMessageScroller(): MessageScrollerControl {
    const control = useContext(MessageScrollerContext);
    if (control === null) {
        throw new TypeError("a message scroller part needs a message scroller around it");
    }

    return control;
}
