import type { IconBodies } from "@destack/icon";
import { Shortcut } from "@destack/locale";

/** The elements whose keys type text. */
const TYPING_TAGS: ReadonlySet<string> = new Set(["INPUT", "TEXTAREA", "SELECT"]);

/** The roles of widgets whose keys type text, after ARIA. */
const TYPING_ROLES: ReadonlySet<string> = new Set(["textbox", "searchbox", "combobox"]);

/** Something a person can do, after VS Code's commands: one list feeds palettes, menus and keys. */
export interface Action {
    /** The action's id, unique among its list's actions. */
    readonly id: string;
    /** The title palettes and menus show. */
    readonly title: string;
    /** The icon beside the title. */
    readonly icon?: IconBodies;
    /** The key combination that runs it, such as `mod+d` or `t`. */
    readonly shortcut?: string;
    /** Whether the action destroys something, which palettes and menus color. */
    readonly isDestructive?: boolean;
    /** Whether it cannot run now, which greys it out and leaves its shortcut to others. */
    readonly isDisabled?: boolean;
    /** Do the action. */
    readonly run: () => Promise<void> | void;
}

/** The verbs of actions. */
export const Action = {
    /** Run the first enabled action a key press's shortcut matches, reporting whether one ran. */
    handle(actions: readonly Action[], event: KeyboardEvent, isCommand: boolean): boolean {
        // find the enabled action the press matches, leaving the press alone without one
        const action = actions.find(
            (each) =>
                each.isDisabled !== true &&
                each.shortcut !== undefined &&
                Shortcut.matches(each.shortcut, event, isCommand),
        );
        if (action === undefined) {
            return false;
        }

        // take the press and run the action
        event.preventDefault();
        void action.run();

        return true;
    },

    /** Report whether a key press goes to a field that takes text, whose keys type. */
    isTyping(event: KeyboardEvent): boolean {
        const target = event.target;

        return (
            target instanceof HTMLElement &&
            (target.isContentEditable ||
                TYPING_TAGS.has(target.tagName) ||
                TYPING_ROLES.has(target.getAttribute("role") ?? ""))
        );
    },
};
