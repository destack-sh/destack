import { type Accessor, onCleanup, useContext } from "solid-js";
import { CommandReference } from "@destack/object";
import type { ObjectReference } from "@destack/sync";
import { t } from "@destack/locale";
import { type Action, defineAction } from "../declare/action.ts";
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
    /** Run the action. */
    readonly run: () => Promise<void> | void;
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

    /** Handle an action while focus and its enablement allow, and return what stops handling it. */
    handle(action: Action, handler: ActionHandler): () => void {
        // add the handler to the action's
        const key = CommandReference.format(action.reference);
        this.#actions.set(key, action);
        this.#handlers.set(key, [...(this.#handlers.get(key) ?? []), handler]);

        return () => {
            const left = (this.#handlers.get(key) ?? []).filter((each) => each !== handler);
            if (left.length === 0) {
                this.#actions.delete(key);
                this.#handlers.delete(key);
            } else {
                this.#handlers.set(key, left);
            }
        };
    }

    /** Report whether an action runs now: an enabled handler of it applies to the focus. */
    has(reference: CommandReference): boolean {
        return this.#chosen(CommandReference.format(reference)) !== undefined;
    }

    /** List the actions that run now. */
    actions(): Action[] {
        return [...this.#actions].flatMap(([key, action]) =>
            this.#chosen(key) === undefined ? [] : [action],
        );
    }

    /** Run an action by the handler the focus chooses. */
    async run(reference: CommandReference): Promise<void> {
        // find the handler the focus chooses
        const key = CommandReference.format(reference);
        const handler = this.#chosen(key);
        if (handler === undefined) {
            throw new TypeError(`the page runs no action '${key}' now`);
        }

        // run it
        await handler.run();
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

    /** Choose an action's handler: the innermost around the focus, else the page's latest. */
    #chosen(key: string): ActionHandler | undefined {
        // take the enabled handlers, and the elements around the focus of those within one
        const focus = document.activeElement;
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
