/** An entry of a browser's session history, as the Navigation API's `currentEntry` has it. */
export interface NavigationEntry {
    /** The entry's URL. */
    readonly url: string;
    /** The entry's position in the session history, as the Navigation API's `index`. */
    readonly index: number;
    /** The state the navigation to the entry kept, as the Navigation API's `getState()`. */
    readonly state: unknown;
}

/** How a navigation moves through history, as the Navigation API's `navigate` options do. */
export interface NavigateOptions {
    /** Add an entry, or replace the current one, adding by default. */
    readonly history?: "push" | "replace";
    /** The state the entry keeps, absent for none. */
    readonly state?: unknown;
}

/** A browser's session history, a subset of the Navigation API with the History API's `go`. */
export interface Navigation {
    /** The current entry. */
    readonly currentEntry: NavigationEntry;
    /** Go to a URL, adding or replacing an entry. */
    readonly navigate: (url: string, options?: NavigateOptions) => void;
    /** Move through history by a number of entries at once, back for a negative one. */
    readonly go: (delta: number) => void;
    /** Call a listener on each change of the current entry until the subscription disposes. */
    readonly on: (listener: (entry: NavigationEntry) => void) => Disposable;
}

/** What the browser's session history keeps in each entry's state: its position and its state. */
interface StampedState {
    /** The entry's position. */
    readonly index: number;
    /** The state the navigation kept. */
    readonly state: unknown;
}

/** The browser's session history through the History API, which every browser has. */
class BrowserNavigation implements Navigation {
    /** The listeners of entry changes. */
    readonly #listeners = new Set<(entry: NavigationEntry) => void>();

    /** The current entry. */
    get currentEntry(): NavigationEntry {
        const { index, state } = BrowserNavigation.#read();

        return { url: location.href, index, state };
    }

    /** Go to a URL, adding or replacing an entry. */
    navigate(url: string, options: NavigateOptions = {}): void {
        // move through history keeping the entry's position and state, then tell the listeners
        const { index } = BrowserNavigation.#read();
        if (options.history === "replace") {
            history.replaceState({ index, state: options.state }, "", url);
        } else {
            history.pushState({ index: index + 1, state: options.state }, "", url);
        }
        this.#emit();
    }

    /** Move through history by a number of entries at once. */
    go(delta: number): void {
        history.go(delta);
    }

    /** Call a listener on each change of the current entry until the subscription disposes. */
    on(listener: (entry: NavigationEntry) => void): Disposable {
        // listen to traversals, stamping an entry other code wrote, and to this navigation's own moves
        const traverse = () => {
            BrowserNavigation.#stamp();
            listener(this.currentEntry);
        };
        this.#listeners.add(listener);
        addEventListener("popstate", traverse);

        return {
            [Symbol.dispose]: () => {
                this.#listeners.delete(listener);
                removeEventListener("popstate", traverse);
            },
        };
    }

    /** Tell each listener the current entry. */
    #emit(): void {
        const entry = this.currentEntry;
        for (const listener of this.#listeners) {
            listener(entry);
        }
    }

    /** Read the current entry's position and state, an unstamped entry at the history's end. */
    static #read(): StampedState {
        const current: unknown = history.state;

        return BrowserNavigation.#isStamped(current)
            ? current
            : { index: history.length - 1, state: current };
    }

    /** Stamp an entry other code wrote with its position, as solid-router stamps its depth. */
    static #stamp(): void {
        if (!BrowserNavigation.#isStamped(history.state)) {
            history.replaceState(BrowserNavigation.#read(), "");
        }
    }

    /** Report whether a history state is one this navigation stamped. */
    static #isStamped(state: unknown): state is StampedState {
        return (
            typeof state === "object" &&
            state !== null &&
            "index" in state &&
            typeof state.index === "number" &&
            "state" in state
        );
    }
}

/** Session histories. */
export const Navigation: { readonly system: Navigation } = {
    /** The browser's session history. */
    system: new BrowserNavigation(),
};
