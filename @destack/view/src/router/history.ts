import type { Navigation, NavigationEntry } from "@destack/environment/browser";
import type { LocationChange, RouterHistory, RouterUtils } from "@solidjs/router";

export { browserHistory, hashHistory, memoryHistory } from "@solidjs/router";
export type { MemoryHistoryAdapter, RouterHistory } from "@solidjs/router";

/** A router's history over a tab's session history, whose leave guards confirm each traversal. */
class NavigationHistory {
    /** The tab's session history. */
    readonly #navigation: Navigation;
    /** The router's leave guards, which the router fills on first use. */
    readonly #beforeLeave: RouterUtils["beforeLeave"] = {};
    /** Whether the router moves the history itself, which it needs no notice of. */
    #isSetting = false;

    /** Route over a tab's session history. */
    constructor(navigation: Navigation) {
        this.#navigation = navigation;
    }

    /** The router's tools: moving through history, and its leave guards. */
    get utils(): Partial<RouterUtils> {
        return { go: (delta) => this.#navigation.go(delta), beforeLeave: this.#beforeLeave };
    }

    /** Read the current entry's path and state. */
    get(): LocationChange {
        const entry = this.#navigation.currentEntry;

        return { value: pathOf(entry), state: entry.state };
    }

    /** Add or replace an entry for the router's own move, then scroll as the move asks. */
    set(next: LocationChange): void {
        // move without telling the router of its own move
        this.#isSetting = true;
        try {
            const history = next.replace === true ? "replace" : "push";
            this.#navigation.navigate(next.value, { history, state: next.state });
        } finally {
            this.#isSetting = false;
        }

        // scroll to the fragment's element, else to the top when the move scrolls
        scrollToFragment(this.#navigation.currentEntry, next.scroll === true);
    }

    /** Tell the router of each traversal its leave guards let through, from now until it stops. */
    listen(notify: () => void): () => void {
        // follow from the current entry, ignoring the move that undoes a held traversal
        let index = this.#navigation.currentEntry.index;
        let isIgnoring = false;
        const subscription = this.#navigation.on((entry) => {
            const delta = entry.index - index;
            index = entry.index;

            // skip the router's own moves and the undoing move
            if (this.#isSetting) {
                return;
            } else if (isIgnoring) {
                isIgnoring = false;
            }
            // undo a traversal a leave guard holds, in one move back
            else if (delta !== 0 && this.#beforeLeave.current?.confirm(delta) === false) {
                isIgnoring = true;
                this.#navigation.go(-delta);
            }
            // tell the router of any other traversal
            else {
                notify();
            }
        });

        return () => subscription[Symbol.dispose]();
    }
}

/** Route over a tab's session history, as `browserHistory` routes over the window's. */
export function navigationHistory(navigation: Navigation): RouterHistory {
    // answer the router's history contract, whose `init` the class names `listen`
    const history = new NavigationHistory(navigation);

    return {
        get: () => history.get(),
        set: (next) => history.set(next),
        init: (notify) => history.listen(notify),
        utils: history.utils,
    };
}

/** Read an entry's path below its origin: its path, query and fragment. */
function pathOf(entry: NavigationEntry): string {
    const url = new URL(entry.url);

    return `${url.pathname}${url.search}${url.hash}`;
}

/** Scroll the page to the element an entry's fragment selects, else to the top when asked. */
function scrollToFragment(entry: NavigationEntry, isToTop: boolean): void {
    // scroll nothing outside a page
    if (typeof document === "undefined") {
        return;
    }

    // scroll to the fragment's element, or to the top
    const fragment = decodeURIComponent(new URL(entry.url).hash.slice(1));
    const element = fragment === "" ? null : document.getElementById(fragment);
    if (element !== null) {
        element.scrollIntoView();
    } else if (isToTop) {
        window.scrollTo(0, 0);
    }
}
