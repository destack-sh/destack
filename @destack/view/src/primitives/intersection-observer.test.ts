import { afterAll, beforeAll, expect, test } from "@destack/test";
import { createRoot, createSignal, flush, NotReadyError } from "solid-js";
import {
    createIntersectionObserver,
    createViewportObserver,
    createVisibilityObserver,
    makeIntersectionObserver,
    withDirection,
    withOccurrence,
} from "./intersection-observer.ts";

/** Every stand-in observer, in creation order. */
const instances: StubObserver[] = [];

/** What a simulated intersection change reports. */
interface Change {
    /** Whether the elements intersect. */
    readonly isIntersecting?: boolean;
    /** The elements' rectangle. */
    readonly boundingClientRect?: DOMRectReadOnly;
}

/** A stand-in for the browser's intersection observer, which the tests trigger. */
class StubObserver implements IntersectionObserver {
    /** The observed root, the viewport. */
    readonly root = null;
    /** The margin around the root. */
    readonly rootMargin = "0px";
    /** The thresholds the observer reports at. */
    readonly thresholds = [0];
    /** The scroll margin around the root. */
    readonly scrollMargin = "0px";
    /** The callback to report changes to. */
    readonly onChange: IntersectionObserverCallback;
    /** The options the observer was made with. */
    readonly options: IntersectionObserverInit | undefined;
    /** The observed elements. */
    elements: Element[] = [];

    /** Keep the callback and options, and remember the observer. */
    constructor(onChange: IntersectionObserverCallback, options?: IntersectionObserverInit) {
        this.onChange = onChange;
        this.options = options;
        instances.push(this);
    }

    /** Observe an element. */
    observe(element: Element): void {
        this.elements.push(element);
    }

    /** Stop observing an element. */
    unobserve(element: Element): void {
        this.elements = this.elements.filter((observed) => observed !== element);
    }

    /** Stop observing every element. */
    disconnect(): void {
        this.elements = [];
    }

    /** Return no pending entries. */
    takeRecords(): IntersectionObserverEntry[] {
        return [];
    }

    /** Report a change of every observed element. */
    change(change: Change = {}): void {
        const entries = this.elements.map((target) => entryOf(target, change));
        this.onChange(entries, this);
    }
}

/** The browser's intersection observer, restored after the tests. */
const original = globalThis.IntersectionObserver;

beforeAll(() => {
    globalThis.IntersectionObserver = StubObserver;
});

afterAll(() => {
    globalThis.IntersectionObserver = original;
});

/** Build an element's entry for a change. */
function entryOf(target: Element, change: Change): IntersectionObserverEntry {
    const rectangle = change.boundingClientRect ?? target.getBoundingClientRect();

    return {
        target,
        time: 0,
        rootBounds: null,
        isIntersecting: change.isIntersecting ?? false,
        intersectionRect: rectangle,
        intersectionRatio: change.isIntersecting === true ? 1 : 0,
        boundingClientRect: rectangle,
    };
}

/** Read the latest stand-in observer. */
function latest(): StubObserver {
    const instance = instances.at(-1);
    if (instance === undefined) {
        throw new Error("no observer was made");
    }

    return instance;
}

/** Make a rectangle at a position. */
function at(top: number, left: number): DOMRectReadOnly {
    return DOMRectReadOnly.fromRect({ x: left, y: top, width: 0, height: 0 });
}

test("make an observer of initial elements that adds, removes, restarts and stops", () => {
    // observe two elements with options
    const div = document.createElement("div");
    const img = document.createElement("img");
    const span = document.createElement("span");
    const options: IntersectionObserverInit = { threshold: 0.6, root: div, rootMargin: "10px" };
    const observed = createRoot((disposeRoot) => {
        const { add, remove, start, stop, instance } = makeIntersectionObserver(
            [div, img],
            () => {},
            options,
        );
        const steps = [[...latest().elements]];

        // add and remove one, then stop and restart
        add(span);
        steps.push([...latest().elements]);
        remove(span);
        stop();
        steps.push([...latest().elements]);
        start();
        steps.push([...latest().elements]);
        disposeRoot();

        return { steps, isLatest: instance === latest(), options: latest().options };
    });

    expect(observed).toEqual({
        steps: [[div, img], [div, img, span], [], [div, img]],
        isLatest: true,
        options,
    });
});

test("pass a made observer's entries and instance to its callback", () => {
    const div = document.createElement("div");
    const img = document.createElement("img");
    const observed = createRoot((disposeRoot) => {
        let targets: Element[] = [];
        let isInstance = false;
        const { instance } = makeIntersectionObserver([div, img], (entries, observer) => {
            targets = entries.map((entry) => entry.target);
            isInstance = observer === instance;
        });
        latest().change();
        disposeRoot();

        return { targets, isInstance };
    });

    expect(observed).toEqual({ targets: [div, img], isInstance: true });
});

test("hold each observed element's latest entry in a slot of its own", () => {
    // observe two elements once the effect runs
    const div = document.createElement("div");
    const img = document.createElement("img");
    const [elements] = createSignal<Element[]>([div, img]);
    const { entries, dispose } = createRoot((disposeRoot) => {
        const [observed] = createIntersectionObserver(elements, { threshold: 0.5 });

        return { entries: observed, dispose: disposeRoot };
    });
    const instance = latest();
    flush();

    // report both entering, then leaving
    instance.change({ isIntersecting: true });
    flush();
    const entered = entries.map((entry) => [entry.target, entry.isIntersecting]);
    instance.change({ isIntersecting: false });
    flush();
    const left = entries.map((entry) => entry.isIntersecting);
    dispose();

    expect([entered, left, instance.options]).toEqual([
        [
            [div, true],
            [img, true],
        ],
        [false, false],
        { threshold: 0.5 },
    ]);
});

test("read an element's visibility as pending until its first entry", () => {
    const div = document.createElement("div");
    const { isVisible, dispose } = createRoot((disposeRoot) => {
        const [, readVisibility] = createIntersectionObserver(() => [div]);

        return { isVisible: readVisibility, dispose: disposeRoot };
    });
    const instance = latest();
    flush();
    const pending = (() => {
        try {
            return isVisible(div);
        } catch (error) {
            return error instanceof NotReadyError ? "pending" : error;
        }
    })();
    instance.change({ isIntersecting: true });
    flush();
    const shown = isVisible(div);
    instance.change({ isIntersecting: false });
    flush();
    const hidden = isVisible(div);
    dispose();

    expect([pending, shown, hidden]).toEqual(["pending", true, false]);
});

test("observe the elements a viewport observer starts from, in each argument form", () => {
    // start from elements with one callback, from pairs, and from accessors of both
    const div = document.createElement("div");
    const img = document.createElement("img");
    const counts = createRoot((disposeRoot) => {
        const forms = [
            createViewportObserver([div, img], () => {}),
            createViewportObserver([
                [div, () => {}],
                [img, () => {}],
            ]),
            createViewportObserver(
                () => [div, img],
                () => {},
            ),
            createViewportObserver(() => [
                [div, () => {}],
                [img, () => {}],
            ]),
        ];
        const observed = forms.map(([, { start, instance }]) => {
            start();

            return instance instanceof StubObserver ? instance.elements.length : -1;
        });
        disposeRoot();

        return observed;
    });

    expect(counts).toEqual([2, 2, 2, 2]);
});

test("pass each element's entry to its own callback, added directly or through a ref", () => {
    // start from pairs, add one directly, and one through a ref factory
    const div = document.createElement("div");
    const img = document.createElement("img");
    const span = document.createElement("span");
    const paragraph = document.createElement("p");
    const seen = new Map<string, Element>();
    const observed = createRoot((disposeRoot) => {
        const [add, { start, remove, instance }] = createViewportObserver(
            [
                [div, (entry) => seen.set("div", entry.target)],
                [img, (entry) => seen.set("img", entry.target)],
            ],
            { threshold: 0.6 },
        );
        start();
        add(span, () => (entry: IntersectionObserverEntry) => seen.set("span", entry.target));
        add((entry) => seen.set("paragraph", entry.target))(paragraph);
        latest().change();

        // stop observing one
        remove(div);
        const elements = instance instanceof StubObserver ? [...instance.elements] : [];
        disposeRoot();

        return { elements, options: latest().options };
    });

    expect([Object.fromEntries(seen), observed]).toEqual([
        { div, img, span, paragraph },
        { elements: [img, span, paragraph], options: { threshold: 0.6 } },
    ]);
});

test("follow one element's visibility, pending until the first entry unless given a start", () => {
    // read pending, and the initial values
    const div = document.createElement("div");
    const observed = createRoot((disposeRoot) => {
        const isVisible = createVisibilityObserver(div, { threshold: 0.6 });
        const instance = latest();
        let pending: unknown;
        try {
            pending = isVisible();
        } catch (error) {
            pending = error instanceof NotReadyError ? "pending" : error;
        }
        const initial = [
            createVisibilityObserver(div, { initialValue: false })(),
            createVisibilityObserver(div, { initialValue: true })(),
        ];

        // follow entering and leaving
        instance.change({ isIntersecting: true });
        flush();
        const shown = isVisible();
        instance.change({ isIntersecting: false });
        flush();
        const hidden = isVisible();
        disposeRoot();

        return { pending, initial, shown, hidden, options: instance.options };
    });

    expect(observed).toEqual({
        pending: "pending",
        initial: [false, true],
        shown: true,
        hidden: false,
        options: { threshold: 0.6 },
    });
});

test("decide visibility through a setter", () => {
    const div = document.createElement("div");
    let isGoal = true;
    const observed = createRoot((disposeRoot) => {
        const isVisible = createVisibilityObserver(
            div,
            {},
            (entry) => entry.target === div && isGoal,
        );
        const instance = latest();
        const values: boolean[] = [];
        for (const next of [true, true, false]) {
            isGoal = next;
            instance.change();
            flush();
            values.push(isVisible());
        }
        disposeRoot();

        return values;
    });

    expect(observed).toEqual([true, true, false]);
});

test("tell a setter where its element stands against the viewport", () => {
    const div = document.createElement("div");
    const occurrences = createRoot((disposeRoot) => {
        const seen: string[] = [];
        createVisibilityObserver(
            div,
            {},
            withOccurrence((entry, { occurrence }) => {
                seen.push(occurrence);

                return entry.isIntersecting;
            }),
        );
        const instance = latest();
        for (const isIntersecting of [false, true, true, false]) {
            instance.change({ isIntersecting });
        }
        disposeRoot();

        return seen;
    });

    expect(occurrences).toEqual(["Outside", "Entering", "Inside", "Leaving"]);
});

test("tell a setter the direction its element moves in", () => {
    const div = document.createElement("div");
    const directions = createRoot((disposeRoot) => {
        const seen: string[][] = [];
        createVisibilityObserver(
            div,
            {},
            withDirection((entry, { directionX, directionY }) => {
                seen.push([directionX, directionY]);

                return entry.isIntersecting;
            }),
        );
        const instance = latest();
        const steps: [boolean, DOMRectReadOnly][] = [
            [false, at(0, 0)],
            [true, at(0, 0)],
            [true, at(15, 15)],
            [false, at(-15, -15)],
            [false, at(15, 15)],
        ];
        for (const [isIntersecting, boundingClientRect] of steps) {
            instance.change({ isIntersecting, boundingClientRect });
        }
        disposeRoot();

        return seen;
    });

    expect(directions).toEqual([
        ["None", "None"],
        ["None", "None"],
        ["Left", "Top"],
        ["Left", "Top"],
        ["Right", "Bottom"],
    ]);
});

test("refuse to observe an element with display: contents", () => {
    const element = document.createElement("div");
    element.style.display = "contents";

    expect(() =>
        createRoot((disposeRoot) => {
            try {
                makeIntersectionObserver([element], () => {});
            } finally {
                disposeRoot();
            }
        }),
    ).toThrow(new TypeError("an element with display: contents has no box to observe"));
});
