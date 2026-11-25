import type { DragManager } from "./drag.ts";
import { DragItem, FILES_TYPE } from "./item.ts";

/** The fields that take dropped text themselves. */
const EDITABLE = "input, textarea, [contenteditable]";

/** The effects the platform allows, as a drop may take them, the first one it can. */
const EFFECTS = ["copy", "link", "move"] as const;

/** The drags the platform runs, such as of files from the desktop. */
export class NativeDrag {
    /** The drags they join. */
    readonly #manager: DragManager;
    /** Whether the drag began on this page. */
    #isOwn = false;
    /** Where the drag entered. */
    #start: { readonly x: number; readonly y: number } | undefined;

    /** Create the platform's drags of a manager. */
    constructor(manager: DragManager) {
        this.#manager = manager;
    }

    /** Follow the drags over a page, until stopped. */
    listen(page: Document): () => void {
        // follow them, leaving a drag once the pointer moves, which it does in none
        const listening = new AbortController();
        const { signal } = listening;
        page.addEventListener("dragstart", () => (this.#isOwn = true), { signal });
        page.addEventListener("dragend", () => this.#end(), { signal });
        page.addEventListener("dragover", (event) => this.#over(event), { signal });
        page.addEventListener("dragleave", (event) => this.#leave(event), { signal });
        page.addEventListener("drop", (event) => this.#drop(event), { signal });
        page.addEventListener("pointermove", () => this.#end(), { signal, passive: true });

        return () => listening.abort();
    }

    /** Follow a drag to the target under it. */
    #over(event: DragEvent): void {
        // leave what another part takes, and text over a field
        const transfer = event.dataTransfer;
        const type = this.#typeOf(event);
        if (transfer === null || event.defaultPrevented || type === undefined) {
            this.#end();

            return;
        }

        // enter where something takes it, and follow it
        const { clientX: x, clientY: y } = event;
        if (this.#start === undefined) {
            if (!this.#manager.enter(type, event.view?.document ?? document)) {
                return;
            }
            this.#start = { x, y };
        }
        this.#manager.follow(x, y, x - this.#start.x, y - this.#start.y, "copy");

        // take it at a target, by an effect its source allows
        if (this.#manager.current()?.target !== undefined) {
            event.preventDefault();
            transfer.dropEffect = effectOf(transfer.effectAllowed);
        }
    }

    /** Leave a drag that leaves the page, as some browsers tell of each element it leaves. */
    #leave(event: DragEvent): void {
        const { clientX: x, clientY: y, view } = event;
        const isInside =
            x > 0 && y > 0 && x < (view?.innerWidth ?? 0) && y < (view?.innerHeight ?? 0);
        if (event.relatedTarget === null && !isInside) {
            this.#cancel();
        }
    }

    /** Drop what a drag brings. */
    #drop(event: DragEvent): void {
        // leave what another part takes
        const transfer = event.dataTransfer;
        const isTaken = event.defaultPrevented || transfer === null;
        if (this.#start === undefined || isTaken || this.#manager.current()?.target === undefined) {
            this.#end();

            return;
        }

        // drop where it aims
        event.preventDefault();
        this.#start = undefined;
        this.#isOwn = false;
        this.#manager.drop("copy", DragItem.of(transfer));
    }

    /** Read the type of a drag, none of text over a field. */
    #typeOf(event: DragEvent): string | undefined {
        // leave out the file a browser makes of a picture of the page
        const types = event.dataTransfer?.types ?? [];
        const type = DragItem.typeOf(
            this.#isOwn ? types.filter((each) => each !== FILES_TYPE) : types,
        );
        const isField = event.target instanceof Element && event.target.closest(EDITABLE) !== null;

        return type !== FILES_TYPE && isField ? undefined : type;
    }

    /** Leave a drag, which may enter again. */
    #cancel(): void {
        if (this.#start !== undefined) {
            this.#start = undefined;
            this.#manager.cancel();
        }
    }

    /** End a drag. */
    #end(): void {
        this.#isOwn = false;
        this.#cancel();
    }
}

/** Read the first effect a source allows. */
function effectOf(allowed: DataTransfer["effectAllowed"]): "copy" | "link" | "move" {
    const isAny = allowed === "all" || allowed === "uninitialized";

    return EFFECTS.find((effect) => isAny || allowed.toLowerCase().includes(effect)) ?? "copy";
}
