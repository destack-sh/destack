import { type ObjectType, TextChange } from "@destack/object";
import type { Submission, TextDocument } from "@destack/object/client";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";
import type { ContextScope } from "../declare/context.ts";
import { useView } from "../page/context.ts";
import { useClient } from "../page/view.ts";
import { ViewContext } from "../declare/context.ts";

/** Where a text field is followed. */
export interface ScopeOptions {
    /** The view's scope with the object, its space by default. */
    readonly scope?: ContextScope;
}

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
    // open the field in the object's scope for the component's lifetime
    const client = useClient(ViewContext.scope(useView(), options.scope ?? "space"));
    const followed = client.text(object, id, field);
    onCleanup(() => void followed.close());

    // follow the text
    const text = createMemo(() => {
        // follow the field until the component goes
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());

        return follow(followed, stopping.signal);
    });

    return {
        text,
        replace: (value) => followed.change(TextChange.between(followed.rope.read().text(), value)),
    };
}

/** Follow a text document's characters until the signal aborts. */
async function* follow(source: TextDocument, signal: AbortSignal): AsyncGenerator<string> {
    for await (const rope of source.watch(signal)) {
        yield rope.read().text();
    }
}
