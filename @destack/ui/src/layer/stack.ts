/** The modal stack of each document, after React Aria's overlay stack. */
const DOCUMENTS = new WeakMap<Document, LayerStack>();

/** The modal stack open in a document, newest last, which notices follow to stay interactive. */
export class LayerStack {
    /** The open modal elements, the topmost last. */
    readonly #modals: HTMLElement[];
    /** The handlers told when the topmost modal changes. */
    readonly #listeners: Set<(top: HTMLElement | undefined) => void>;

    /** Create the stack of one document. */
    constructor() {
        this.#modals = [];
        this.#listeners = new Set();
    }

    /** Read the stack of an element's document. */
    static of(element: Element): LayerStack {
        // reuse the document's stack
        const document = element.ownerDocument;
        const known = DOCUMENTS.get(document);
        if (known !== undefined) {
            return known;
        }

        // start the document's stack
        const stack = new LayerStack();
        DOCUMENTS.set(document, stack);

        return stack;
    }

    /** Read the topmost open modal, none while the page is the top. */
    top(): HTMLElement | undefined {
        return this.#modals.at(-1);
    }

    /** Add a modal that just opened on top. */
    open(element: HTMLElement): void {
        this.#remove(element);
        this.#modals.push(element);
        this.#notify();
    }

    /** Remove a modal that closed or unmounted, wherever it was in the stack. */
    close(element: HTMLElement): void {
        if (this.#remove(element)) {
            this.#notify();
        }
    }

    /** Follow the topmost modal until the returned function stops it. */
    subscribe(listener: (top: HTMLElement | undefined) => void): () => void {
        this.#listeners.add(listener);

        return () => this.#listeners.delete(listener);
    }

    /** Take a modal out of the stack, reporting whether it was in it. */
    #remove(element: HTMLElement): boolean {
        const index = this.#modals.indexOf(element);
        if (index !== -1) {
            this.#modals.splice(index, 1);
        }

        return index !== -1;
    }

    /** Tell every handler the topmost modal. */
    #notify(): void {
        const top = this.top();
        for (const listener of this.#listeners) {
            listener(top);
        }
    }
}
