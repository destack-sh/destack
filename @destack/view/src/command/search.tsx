import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import type { Identifier } from "@destack/schema";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandShortcut,
    useCommand,
} from "@destack/ui/command";
import { CommandReference } from "../declare/command.ts";
import type { JSX } from "../solid/component.ts";
import { For, Show } from "../solid/flow.ts";
import { createMemo } from "../solid/reactive.ts";
import {
    type CommandEntry,
    type ObjectEntry,
    Palette,
    type PaletteClient,
    type PaletteEntry,
    type PaletteFocus,
    type PaletteListing,
    type ViewEntry,
    type WindowEntry,
} from "../palette/entry.ts";
import type { PaletteRun } from "./run.ts";

/** Search the palette's entries, running a command, opening a view, narrowing to a space or focusing a window. */
export function SearchStep(properties: {
    readonly client: PaletteClient;
    readonly focus: PaletteFocus;
    readonly running: PaletteRun;
    readonly listing: PaletteListing;
}): JSX.Element {
    const { client, running } = properties;

    return (
        <Command shouldFilter={false}>
            <PaletteSearch
                listing={properties.listing}
                focus={properties.focus}
                scope={running.scope()}
                onCommand={(command) => running.run(command)}
                onOpen={(entry) => void client.open(entry).catch(reportError)}
                onSpace={(space) => running.narrow(space)}
                onWindow={(window) => {
                    client.focus(window).then(() => client.close(), reportError);
                }}
            />
        </Command>
    );
}

/** The choices a palette's search hands on: a command to run, a view or object to open, a space to narrow to, a window to focus. */
interface SearchChoices {
    /** Run a command. */
    readonly onCommand: (command: CommandEntry) => void;
    /** Open a view, or an object in the view presenting it. */
    readonly onOpen: (entry: ViewEntry | ObjectEntry) => void;
    /** Narrow the search to a space. */
    readonly onSpace: (space: Identifier<"space">) => void;
    /** Bring a window into focus. */
    readonly onWindow: (window: WindowEntry) => void;
}

/** Render the search: the ranked entries matching the typed text, narrowed to a space when one is chosen, and the spaces the palette could not read. */
function PaletteSearch(
    properties: SearchChoices & {
        readonly listing: PaletteListing;
        readonly focus: PaletteFocus;
        readonly scope: Identifier<"space"> | undefined;
    },
): JSX.Element {
    // rank the entries of the space chosen, or of every space, by the typed text and the focus
    const locale = useLocale();
    const control = useCommand();
    const ranked = createMemo(() => {
        const { scope } = properties;
        const shown = properties.listing.entries.filter(
            (entry) => scope === undefined || Palette.space(entry) === scope,
        );

        return Palette.rank(shown, control.search(), properties.focus);
    });

    // title the entries' spaces by the space entries
    const spaces = createMemo(
        () =>
            new Map(
                properties.listing.entries.flatMap((entry) =>
                    entry.kind === "space" ? [[entry.id, entry.title] as const] : [],
                ),
            ),
    );

    return (
        <>
            <CommandInput placeholder={locale.render(t`Run a command or open a view`)} />
            <Show when={properties.scope}>
                {(scope) => <div data-slot="command-scope">{spaces().get(scope()) ?? scope()}</div>}
            </Show>
            <CommandList>
                <CommandEmpty>{locale.render(t`No commands`)}</CommandEmpty>
                <EntryGroups {...properties} ranked={ranked()} spaces={spaces()} />
            </CommandList>
            <Show when={properties.listing.failures.length > 0}>
                <ul data-slot="command-failures">
                    <For each={properties.listing.failures}>{(failure) => <li>{failure}</li>}</For>
                </ul>
            </Show>
        </>
    );
}

/** Render the ranked entries in one group per kind: commands, windows, views, recent objects and spaces. */
function EntryGroups(
    properties: SearchChoices & {
        readonly ranked: readonly PaletteEntry[];
        readonly spaces: ReadonlyMap<string, string>;
    },
): JSX.Element {
    const locale = useLocale();
    const kind = <Kind extends PaletteEntry["kind"]>(wanted: Kind) =>
        properties.ranked.filter(
            (entry): entry is Extract<PaletteEntry, { kind: Kind }> => entry.kind === wanted,
        );

    return (
        <>
            <EntryGroup
                heading={locale.render(t`Commands`)}
                entries={kind("command")}
                spaces={properties.spaces}
                onSelect={properties.onCommand}
            />
            <EntryGroup
                heading={locale.render(t`Windows`)}
                entries={kind("window")}
                spaces={properties.spaces}
                onSelect={properties.onWindow}
            />
            <EntryGroup
                heading={locale.render(t`Views`)}
                entries={kind("view")}
                spaces={properties.spaces}
                onSelect={properties.onOpen}
            />
            <EntryGroup
                heading={locale.render(t`Recent`)}
                entries={kind("object")}
                spaces={properties.spaces}
                onSelect={properties.onOpen}
            />
            <EntryGroup
                heading={locale.render(t`Spaces`)}
                entries={kind("space")}
                spaces={properties.spaces}
                onSelect={(entry) => properties.onSpace(entry.id)}
            />
        </>
    );
}

/** Render a group of entries of one kind, each with its space and keybinding. */
function EntryGroup<Entry extends PaletteEntry>(properties: {
    readonly heading: string;
    readonly entries: readonly Entry[];
    readonly spaces: ReadonlyMap<string, string>;
    readonly onSelect: (entry: Entry) => void;
}): JSX.Element {
    return (
        <CommandGroup heading={properties.heading}>
            <For each={properties.entries}>
                {(entry) => (
                    <CommandItem value={valueOf(entry)} onSelect={() => properties.onSelect(entry)}>
                        {entry.title}
                        <Show when={"space" in entry ? properties.spaces.get(entry.space) : null}>
                            {(space) => <span data-slot="command-space">{space()}</span>}
                        </Show>
                        <Show when={entry.kind === "command" ? entry.keybinding : null}>
                            {(keybinding) => <CommandShortcut>{keybinding()}</CommandShortcut>}
                        </Show>
                    </CommandItem>
                )}
            </For>
        </CommandGroup>
    );
}

/** Name an entry among the options of its group, unique across the palette. */
function valueOf(entry: PaletteEntry): string {
    if (entry.kind === "command") {
        return `${entry.space}/${CommandReference.format(entry)}`;
    } else if (entry.kind === "view") {
        return `${entry.space}/${entry.installation}/${entry.name}`;
    } else if (entry.kind === "object") {
        return `${entry.space}/${entry.object.type}/${entry.object.id}`;
    }

    return entry.id;
}
