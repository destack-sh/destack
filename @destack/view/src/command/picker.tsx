import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import {
    CommandEmpty,
    CommandInput,
    CommandItem,
    CommandList,
    useCommand,
} from "@destack/ui/command";
import type { JSX } from "../solid/component.ts";
import { For } from "../solid/flow.ts";
import { createEffect, createSignal } from "../solid/reactive.ts";
import type { CommandEntry, PaletteClient, PaletteObject } from "../palette/entry.ts";

/** Render the picker of a command's object, listing the objects whose title matches the typed text. */
export function ObjectPicker(properties: {
    readonly client: PaletteClient;
    readonly command: CommandEntry;
    readonly onObject: (id: string) => void;
}): JSX.Element {
    // keep the objects the latest text found
    const locale = useLocale();
    const control = useCommand();
    const [objects, setObjects] = createSignal<readonly PaletteObject[]>([]);

    // list the objects again whenever the text changes, keeping the latest answer
    let asked = 0;
    createEffect(
        () => control.search(),
        (title) => {
            asked += 1;
            const question = asked;
            properties.client.objects(properties.command, title).then((found) => {
                if (question === asked) {
                    setObjects(found);
                }
            }, reportError);
        },
    );

    return (
        <>
            <CommandInput placeholder={locale.render(t`Find the object`)} />
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
