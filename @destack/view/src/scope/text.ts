import { type Undo, type ObjectType, TextChange } from "@destack/object";
import { LiveText, type Submission } from "@destack/object/client";
import { type PermissionScope } from "@destack/access";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";
import { requireScope, useClient, useView } from "../page/view.ts";

/** Where a text field is followed. */
export interface ScopeOptions {
    /** The view's scope with the object, its space by default. */
    readonly scope?: PermissionScope;
}

/** A text field followed live and replaced by whole values. */
export interface TextField {
    /** The current text, local predictions included, pending until the replica has it. */
    readonly text: Accessor<string>;
    /** Replace the text with a new value, as the one change between them. */
    replace(value: string): Promise<Submission<Undo>> | undefined;
}

/** Follow one text field of an object in the view's scope that keeps it. */
export function useText(
    object: ObjectType,
    id: string,
    field: string,
    options: ScopeOptions = {},
): TextField {
    // open the field in the object's scope for the component's lifetime
    const client = useClient(requireScope(useView(), options.scope ?? "space"));
    const live = LiveText.open(client, object, id, field);
    onCleanup(() => void live.close());

    // follow the text
    const text = createMemo(() => {
        // follow the field until the component goes
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return follow(live, stopping.signal);
    });

    return {
        text,
        replace: (value) => {
            // send the one change between the shown text and the new one
            const change = TextChange.between(text(), value);

            return change.from === change.to && change.insert === ""
                ? undefined
                : live.change(change);
        },
    };
}

/** Follow a live text's value until the signal aborts. */
async function* follow(live: LiveText, signal: AbortSignal): AsyncGenerator<string> {
    for await (const sequence of live.watch(signal)) {
        yield sequence.text();
    }
}
