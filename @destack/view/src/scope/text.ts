import { type ObjectType, TextChange } from "@destack/object";
import type { Submission, TextDocument } from "@destack/object/client";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";
import { type ScopeOptions, useNode } from "./query.ts";

/** A text field followed live and replaced by whole values. */
export interface TextField {
    /** The current text, local predictions included, pending until the replica has it. */
    readonly text: Accessor<string>;
    /** Replace the text with a new value, as the one change between them. */
    readonly replace: (value: string) => Submission<void>;
}

/** Follow one text field of an object in the view's scope that keeps it. */
export function useText(
    object: ObjectType,
    id: string,
    field: string,
    options: ScopeOptions = {},
): TextField {
    // open the field on the node of the object's scope for the component's lifetime
    const nodes = useNode(object, options);
    const opened = createMemo(async () => {
        // register the close while the memo owns its cleanup, before the node opens
        const closing = new AbortController();
        onCleanup(() => closing.abort());
        const followed = (await nodes().node).text(object, id, field);
        if (closing.signal.aborted) {
            await followed.close();
        } else {
            closing.signal.addEventListener("abort", () => void followed.close(), { once: true });
        }

        return followed;
    });

    // follow the text
    const text = createMemo(() => {
        // follow the field until it reopens or the component goes
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return follow(opened(), stopping.signal);
    });

    return {
        text,
        replace: (value) => {
            // replace the opened field's text
            const followed = opened();

            return followed.change(TextChange.between(followed.rope.read().text(), value));
        },
    };
}

/** Follow a text document's characters until the signal aborts. */
async function* follow(source: TextDocument, signal: AbortSignal): AsyncGenerator<string> {
    for await (const rope of source.watch(signal)) {
        yield rope.read().text();
    }
}
