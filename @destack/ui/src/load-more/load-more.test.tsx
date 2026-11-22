import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { LoadMore } from "./load-more.tsx";

/** The observers created during a test, each with what it watches and how. */
interface Watch {
    /** Report the watched element as in or out of view. */
    readonly report: (isIntersecting: boolean) => void;
    /** The margin around the view. */
    readonly rootMargin: string | undefined;
    /** Whether the observer still watches. */
    isWatching: boolean;
}

/** Stand in for the browser's intersection observer, until the test ends. */
function observeIntersections(): Watch[] {
    // record each observer the element creates
    const watches: Watch[] = [];
    const original = globalThis.IntersectionObserver;
    class Observer implements IntersectionObserver {
        /** The element the view is, the page's viewport when null. */
        readonly root = null;
        /** The margin around the view. */
        readonly rootMargin: string;
        /** The margin around the scrolling ancestors. */
        readonly scrollMargin = "";
        /** The shares in view that report. */
        readonly thresholds: readonly number[] = [];
        /** The record of this observer. */
        readonly #watch: Watch;
        /** The watched element. */
        #target: Element = document.body;

        /** Record an observer. */
        constructor(callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
            this.rootMargin = options?.rootMargin ?? "";
            this.#watch = {
                rootMargin: options?.rootMargin,
                isWatching: false,
                report: (isIntersecting) => {
                    const entry: IntersectionObserverEntry = {
                        isIntersecting,
                        intersectionRatio: isIntersecting ? 1 : 0,
                        target: this.#target,
                        time: 0,
                        boundingClientRect: new DOMRect(),
                        intersectionRect: new DOMRect(),
                        rootBounds: null,
                    };
                    callback([entry], this);
                },
            };
            watches.push(this.#watch);
        }

        /** Start watching. */
        observe(target: Element): void {
            this.#target = target;
            this.#watch.isWatching = true;
        }

        /** Stop watching one element. */
        unobserve(): void {
            this.#watch.isWatching = false;
        }

        /** Stop watching. */
        disconnect(): void {
            this.#watch.isWatching = false;
        }

        /** Read the reports not yet delivered. */
        takeRecords(): IntersectionObserverEntry[] {
            return [];
        }
    }
    globalThis.IntersectionObserver = Observer;
    onTestFinished(() => {
        globalThis.IntersectionObserver = original;
    });

    return watches;
}

test("load more as the element comes near the view, wait while loading, and watch again after", () => {
    // come into view, start loading, finish, and come into view again
    const watches = observeIntersections();
    const [isLoading, setLoading] = createSignal(false);
    let loads = 0;
    const { container } = render(() => (
        <LoadMore
            onLoadMore={() => {
                loads += 1;
                setLoading(true);
            }}
            isLoading={isLoading()}
            scrollOffset={2}
        >
            Loading
        </LoadMore>
    ));
    flush();
    watches[0]?.report(false);
    watches[0]?.report(true);
    flush();
    const loading = [container.textContent, watches[0]?.isWatching, watches.length];
    setLoading(false);
    flush();
    watches[1]?.report(true);
    flush();

    expect({ loads, loading, margin: watches[0]?.rootMargin, watches: watches.length }).toEqual({
        loads: 2,
        loading: ["Loading", false, 1],
        margin: "200%",
        watches: 2,
    });
});
