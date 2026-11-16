import type { Action, ActionDescription } from "../declare/action.ts";

/** Describe an action by its title and keys. */
export function describeAction(action: Action): ActionDescription {
    return action.describe();
}
