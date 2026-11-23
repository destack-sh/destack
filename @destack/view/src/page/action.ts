import { isServer } from "@solidjs/web";
import { DeclarationReference } from "@destack/package/declare";
import { type Accessor, createSignal, onCleanup, type Setter, useContext } from "solid-js";
import type { ObjectReference } from "@destack/sync";
import { t } from "@destack/locale";
import { type Action, defineAction } from "@destack/object";
import { ViewMountContext } from "./context.ts";

/** Undo the person's newest change across the page's scopes. */
export const undo = defineAction({
    name: "undo",
    title: t`Undo`,
    category: t`Edit`,
    keybinding: "mod+z",
});

/** Redo the person's newest undone change across the page's scopes. */
export const redo = defineAction({
    name: "redo",
    title: t`Redo`,
    category: t`Edit`,
    keybinding: "mod+shift+z",
});

/** Open the command palette over the page, from anywhere in it. */
export const palette = defineAction({
    name: "palette",
    title: t`Command Palette`,
    keybinding: "mod+k",
    focus: "any",
});

/** What runs an action, and when: while focus is within an element, and while it is enabled. */
export interface ActionHandler {
    /** Run the action where the focus was. */
    readonly run: (focus: Element | null) => Promise<void> | void;
    /** The element focus is within for the handler to run, as the responder chain picks it. */
    readonly within?: Accessor<Element | undefined>;
    /** Whether the handler runs now, read reactively, as a menu item validates. */
    readonly isEnabled?: Accessor<boolean>;
}

/** The actions a running page's components handle, and the objects the person selects in it. */
export class ActionRegistry {
    /** The handled actions, by setting key. */
    readonly #actions = new Map<string, Action>();
    /** The handlers of each action, by setting key, the latest last. */
    readonly #handlers = new Map<string, ActionHandler[]>();
    /** Read the objects the person selected in the page, absent while no component tracks them. */
    #selection: (() => readonly ObjectReference[]) | undefined;
    /** Count the changes to the handlers, which readers of the actions follow. */
    readonly #revision: Accessor<number>;
    /** Replace the count of changes. */
    readonly #setRevision: Setter<number>;

    /** Create a page's registry without handlers. */
    constructor() {
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#revision = revision;
        this.#setRevision = setRevision;
    }

    /** Handle an action while focus and its enablement allow, and return what stops handling it. */
    handle(action: Action, handler: ActionHandler): () => void {
        // add the handler to the action's
        const key = DeclarationReference.key(action.reference);
        this.#actions.set(key, action);
        this.#handlers.set(key, [...(this.#handlers.get(key) ?? []), handler]);
        this.#changed();

        return () => {
            const left = (this.#handlers.get(key) ?? []).filter((each) => each !== handler);
            if (left.length === 0) {
                this.#actions.delete(key);
                this.#handlers.delete(key);
            } else {
                this.#handlers.set(key, left);
            }
            this.#changed();
        };
    }

    /** Report whether an action runs now: an enabled handler of it applies to the focus. */
    has(reference: DeclarationReference, focus: Element | null): boolean {
        return this.#chosen(DeclarationReference.key(reference), focus) !== undefined;
    }

    /** List the actions that run for a focus, as handlers come and go. */
    actions(focus: Element | null): Action[] {
        return [...this.#actions].flatMap(([key, action]) =>
            this.#chosen(key, focus) === undefined ? [] : [action],
        );
    }

    /** Run an action by the handler a focus chooses, handing it the focus. */
    async run(reference: DeclarationReference, focus: Element | null): Promise<void> {
        // find the handler the focus chooses
        const key = DeclarationReference.key(reference);
        const handler = this.#chosen(key, focus);
        if (handler === undefined) {
            throw new TypeError(`the page runs no action '${key}' now`);
        }

        // run it where the focus was
        await handler.run(focus);
    }

    /** Track the objects the person selects, refusing a second component tracking them. */
    select(read: () => readonly ObjectReference[]): () => void {
        // refuse two components tracking one selection
        if (this.#selection !== undefined) {
            throw new TypeError("the page already tracks a selection");
        }
        this.#selection = read;

        return () => {
            if (this.#selection === read) {
                this.#selection = undefined;
            }
        };
    }

    /** Read the objects the person selected in the page, none while no component tracks them. */
    selection(): ObjectReference[] {
        return [...(this.#selection?.() ?? [])];
    }

    /** Tell readers of the actions that the handlers changed. */
    #changed(): void {
        // write no signal on a server, which renders once
        if (!isServer) {
            this.#setRevision((count) => count + 1);
        }
    }

    /** Choose an action's handler: the innermost around the focus, else the page's latest. */
    #chosen(key: string, focus: Element | null): ActionHandler | undefined {
        // follow the handlers as they come and go
        this.#revision();

        // take the enabled handlers, and the elements around the focus of those within one
        const enabled = (this.#handlers.get(key) ?? []).filter(
            (handler) => handler.isEnabled?.() ?? true,
        );
        const around = enabled.flatMap((handler) => {
            const element = handler.within?.();

            return element !== undefined && focus !== null && element.contains(focus)
                ? [{ handler, element }]
                : [];
        });

        // choose the innermost around the focus, else the latest handler for the whole page
        const innermost = around.find(({ element }) =>
            around.every((other) => other.element === element || other.element.contains(element)),
        );

        return innermost?.handler ?? enabled.findLast((handler) => handler.within === undefined);
    }
}

/** Read the actions the running page's components handle. */
export function useActions(): ActionRegistry {
    return useContext(ViewMountContext).actions;
}

/** Handle an action of the running page while the calling component is mounted. */
export function handleAction(action: Action, handler: ActionHandler): void {
    // handle it until the component goes
    const stop = useContext(ViewMountContext).actions.handle(action, handler);
    onCleanup(stop);
}

/** Track the objects the person selects in the running page while the component is mounted. */
export function registerSelection(read: () => readonly ObjectReference[]): void {
    // track it until the component goes
    const stop = useContext(ViewMountContext).actions.select(read);
    onCleanup(stop);
}
