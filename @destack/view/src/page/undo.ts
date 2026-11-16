import type { UndoStack } from "@destack/object/client";
import { type Accessor, createSignal, type Setter } from "solid-js";

/** The events starting a new gesture, which closes every open group. */
const GESTURES = ["pointerdown", "focusin", "paste", "drop", "cut"] as const;

/** A page's undo across the stacks of the scopes it opens. */
export class PageUndo {
    /** The undo stacks of the page's clients. */
    readonly stacks: readonly UndoStack[];
    /** Whether any stack has a group to undo. */
    readonly canUndo: Accessor<boolean>;
    /** Whether any stack has a group to redo. */
    readonly canRedo: Accessor<boolean>;
    /** Set what each stack can undo and redo, by stack. */
    readonly #setStates: Setter<
        readonly { readonly canUndo: boolean; readonly canRedo: boolean }[]
    >;

    /** Undo across some stacks, nothing to undo or redo until their first reads arrive. */
    constructor(stacks: readonly UndoStack[]) {
        // start with nothing to undo or redo on any stack
        this.stacks = stacks;
        const [states, setStates] = createSignal<
            readonly { readonly canUndo: boolean; readonly canRedo: boolean }[]
        >(stacks.map(() => ({ canUndo: false, canRedo: false })));
        this.canUndo = () => states().some((state) => state.canUndo);
        this.canRedo = () => states().some((state) => state.canRedo);
        this.#setStates = setStates;
    }

    /** Undo the latest group of any stack, resolving whether there was one. */
    async undo(): Promise<boolean> {
        const stack = await this.#latest("done");

        return stack === undefined ? false : stack.undo().predicted;
    }

    /** Redo the latest undone group of any stack, resolving whether there was one. */
    async redo(): Promise<boolean> {
        const stack = await this.#latest("undone");

        return stack === undefined ? false : stack.redo().predicted;
    }

    /** Follow what each stack can undo and redo, and close groups at gestures, until aborted. */
    listen(signal: AbortSignal): void {
        // follow each stack's undo and redo
        for (const [position, stack] of this.stacks.entries()) {
            void (async () => {
                for await (const state of stack.watch(signal)) {
                    this.#setStates((states) => states.with(position, state));
                }
            })().catch((error: unknown) => globalThis.reportError(error));
        }

        // close every open group as a gesture starts
        const close = () => {
            for (const stack of this.stacks) {
                void stack.close().catch((error: unknown) => globalThis.reportError(error));
            }
        };
        for (const gesture of GESTURES) {
            window.addEventListener(gesture, close, { capture: true, signal });
        }
    }

    /** Find the stack whose latest group in a state is newest. */
    async #latest(state: "done" | "undone"): Promise<UndoStack | undefined> {
        // read each stack's latest entry
        const entries = await Promise.all(
            this.stacks.map(async (stack) => ({ stack, entry: await stack.latest(state) })),
        );

        // take the newest
        const newest = entries
            .flatMap(({ stack, entry }) =>
                entry === undefined ? [] : [{ stack, time: entry.time }],
            )
            .toSorted((left, right) => right.time - left.time)
            .at(0);

        return newest?.stack;
    }
}
