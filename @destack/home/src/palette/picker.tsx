import { t } from "@destack/locale";
import {
    CommandEmpty,
    CommandInput,
    CommandItem,
    CommandList,
    useCommand,
} from "@destack/ui/command";
import { createEffect, createSignal, For, type JSX, useLocale } from "@destack/view";
import type { CommandEntry } from "@destack/view/palette";
import type { PickedObject, SpaceCatalog } from "./catalog.ts";

/** Render the picker of a command's target, listing the objects it applies to by title. */
export function ObjectPicker(properties: {
    readonly catalog: SpaceCatalog;
    readonly command: CommandEntry;
    readonly onObject: (id: string) => void;
}): JSX.Element {
    // show the objects the latest text found
    const locale = useLocale();
    const control = useCommand();
    const [objects, setObjects] = createSignal<readonly PickedObject[]>([]);

    // list the objects again whenever the text changes, showing the latest answer
    let asked = 0;
    createEffect(
        () => control.search(),
        (title) => {
            asked += 1;
            const question = asked;
            properties.catalog.objects(properties.command, title).then(
                (found) => {
                    if (question === asked) {
                        setObjects(found);
                    }
                },
                (error: unknown) => globalThis.reportError(error),
            );
        },
    );

    return (
        <>
            {/* Title search */}
            <CommandInput placeholder={locale.render(t`Find the object`)} />

            {/* Objects the command applies to */}
            <CommandList>
                <CommandEmpty>{locale.render(t`No objects`)}</CommandEmpty>
                <For each={objects()}>
                    {(object) => (
                        <CommandItem
                            value={object.id}
                            onSelect={() => properties.onObject(object.id)}
                        >
                            {object.title}
                        </CommandItem>
                    )}
                </For>
            </CommandList>
        </>
    );
}
