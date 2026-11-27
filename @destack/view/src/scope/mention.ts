import { Choice } from "@destack/object";
import type { ObjectNode } from "@destack/object/client";
import type { Subject } from "@destack/sync";
import { useMount } from "../page/context.ts";
import { useLocale } from "../page/locale.ts";
import { type Accessor, createMemo, onCleanup } from "../solid/reactive.ts";

/** Follow a mentioned subject as a choice: its title, icon and hue as its type presents it. */
export function useMention(subject: Subject): Accessor<Choice> {
    const locale = useLocale();

    return createMemo(async (): Promise<Choice> => {
        // read the mount before any await, where it suspends an unmounted view, then take the node of the subject's scope
        const mount = useMount();
        const opened = mount.clients.get(subject.scope);
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());
        const node: ObjectNode = opened ?? (await mount.open(subject.scope, stopping.signal));

        // read its row through the node holding its type, unresolved for a type or row it lacks
        const type = node.objects.find(
            (each) => each.package.id === subject.packageId && each.name === subject.type,
        );
        const row = type === undefined ? undefined : await node.find(type, subject);

        return type === undefined || row === undefined
            ? { value: subject, title: subject.id }
            : Choice.object(type, subject, row, node.context().moment, locale);
    });
}
