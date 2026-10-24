/** An entry of a browser's session history, as the Navigation API's `currentEntry` has it. */
export interface NavigationEntry {
    /** The entry's URL. */
    readonly url: string;
}

/** How a navigation moves through history, as the Navigation API's `history` option does. */
export interface NavigateOptions {
    /** Add an entry, or replace the current one, adding by default. */
    readonly history?: "push" | "replace";
}

/** A browser's session history, a subset of the Navigation API. */
export interface Navigation {
    /** The current entry. */
    readonly currentEntry: NavigationEntry;
    /** Go to a URL, adding or replacing an entry. */
    readonly navigate: (url: string, options?: NavigateOptions) => void;
    /** Go back one entry. */
    readonly back: () => void;
    /** Go forward one entry. */
    readonly forward: () => void;
    /** Call a listener on each change of the current entry until the subscription disposes. */
    readonly on: (listener: (entry: NavigationEntry) => void) => Disposable;
}

/** The browser's session history through the History API, which every browser has. */
class BrowserNavigation implements Navigation {
    /** The listeners of entry changes. */
    readonly #listeners = new Set<(entry: NavigationEntry) => void>();

    /** The current entry. */
    get currentEntry(): NavigationEntry {
        return { url: location.href };
    }

    /** Go to a URL, adding or replacing an entry. */
    navigate(url: string, options: NavigateOptions = {}): void {
        // move through history, then tell the listeners
        if (options.history === "replace") {
            history.replaceState(null, "", url);
        } else {
            history.pushState(null, "", url);
        }
        this.#emit();
    }

    /** Go back one entry. */
    back(): void {
        history.back();
    }

    /** Go forward one entry. */
    forward(): void {
        history.forward();
    }

    /** Call a listener on each change of the current entry until the subscription disposes. */
    on(listener: (entry: NavigationEntry) => void): Disposable {
        // listen to traversals, and to this navigation's own moves
        const traverse = () => listener(this.currentEntry);
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
}

/** Session histories. */
export const Navigation: { readonly system: Navigation } = {
    /** The browser's session history. */
    system: new BrowserNavigation(),
};
