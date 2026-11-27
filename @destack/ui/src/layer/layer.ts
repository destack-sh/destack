import type { Scheduler } from "@destack/environment";

/** The top layer of each document. */
const DOCUMENTS = new WeakMap<Document, TopLayer>();

/** The top layer of a document, as the platform keeps it: modals in order, one show at a time. */
export class TopLayer {
    /** The open modal elements, the topmost last. */
    readonly #modals: HTMLElement[];
    /** The handlers told when the topmost modal changes. */
    readonly #listeners: Set<(top: HTMLElement | undefined) => void>;
    /** Whether a show runs now, inside which the document runs no other. */
    #isShowing: boolean;
    /** The popovers whose show or hide the platform runs now, each by its beforetoggle. */
    readonly #held: Map<HTMLElement, Event>;
    /** The shows asked for while another ran, each with whether its overlay still wants it. */
    readonly #queued: {
        readonly element: HTMLElement;
        readonly show: () => void;
        readonly isWanted: () => boolean;
    }[];
    /** The popovers whose toggles the top layer follows. */
    readonly #watched: WeakSet<HTMLElement>;

    /** Create the top layer of one document. */
    constructor() {
        // start without modals, handlers or shows
        this.#modals = [];
        this.#listeners = new Set();
        this.#isShowing = false;
        this.#held = new Map();
        this.#queued = [];
        this.#watched = new WeakSet();
    }

    /** Read the top layer of an element's document. */
    static of(element: Element): TopLayer {
        // reuse the document's top layer
        const document = element.ownerDocument;
        const known = DOCUMENTS.get(document);
        if (known !== undefined) {
            return known;
        }

        // start the document's top layer
        const layer = new TopLayer();
        DOCUMENTS.set(document, layer);

        return layer;
    }

    /** Read the topmost open modal, none while the page is the top. */
    top(): HTMLElement | undefined {
        return this.#modals.at(-1);
    }

    /** Show a dialog modally, on top of the modals open before it. */
    showModal(dialog: HTMLDialogElement): void {
        // show it, then put it on top of the modals
        dialog.showModal();
        this.#remove(dialog);
        this.#modals.push(dialog);
        this.#notify();
    }

    /** Close a dialog where it is open, and take it out of the modals wherever it was. */
    close(dialog: HTMLDialogElement): void {
        // close it, as the platform may have already
        if (dialog.open) {
            dialog.close();
        }

        // tell the modal's followers where it was one
        if (this.#remove(dialog)) {
            this.#notify();
        }
    }

    /** Show a popover now, or after the show or hide running, while its overlay is still open. */
    showPopover(element: HTMLElement, show: () => void, isOpen: () => boolean): void {
        // wait for the show or hide running, as one that runs inside it throws
        const isWanted = (): boolean => isOpen() && !element.matches(":popover-open");
        if (this.#isBusy()) {
            this.#queued.push({ element, show, isWanted });
            return;
        }

        // show it, then the shows asked for meanwhile
        this.#isShowing = true;
        try {
            show();
        } finally {
            this.#isShowing = false;
        }
        this.#drain();
    }

    /** Follow a popover's toggles, holding other shows while the platform shows or hides it. */
    watch(element: HTMLElement, scheduler: Scheduler): void {
        // follow each popover once, its listeners going with it
        if (this.#watched.has(element)) {
            return;
        }
        this.#watched.add(element);

        // hold from its beforetoggle to its toggle, a refused show or a removal ending it sooner
        element.addEventListener("beforetoggle", (event) => this.#hold(element, event, scheduler));

        // end the hold before the overlay follows its toggle, as capturing listeners run first
        element.addEventListener("toggle", () => this.#settle(element), { capture: true });
    }

    /** Hold the document's shows while the platform shows or hides a popover, at most a task. */
    #hold(element: HTMLElement, event: Event, scheduler: Scheduler): void {
        if (!this.#held.has(element)) {
            this.#held.set(element, event);
            void scheduler.yield().then(() => this.#settle(element));
        }
    }

    /** Tell whether a show or hide runs, a show its owner refused running none. */
    #isBusy(): boolean {
        return this.#isShowing || [...this.#held.values()].some((event) => !event.defaultPrevented);
    }

    /** End a popover's show or hide, then run the shows asked for meanwhile. */
    #settle(element: HTMLElement): void {
        if (this.#held.delete(element)) {
            this.#drain();
        }
    }

    /** Run the shows asked for meanwhile that are still wanted, once none runs. */
    #drain(): void {
        if (this.#isBusy()) {
            return;
        }
        for (const next of this.#queued.splice(0)) {
            if (next.isWanted()) {
                this.showPopover(next.element, next.show, () => next.isWanted());
            }
        }
    }

    /** Follow the topmost modal until the returned function stops it. */
    subscribe(listener: (top: HTMLElement | undefined) => void): () => void {
        this.#listeners.add(listener);

        return () => this.#listeners.delete(listener);
    }

    /** Take a modal out of the modals, reporting whether it was among them. */
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
