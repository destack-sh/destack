import { t } from "@destack/locale";
import { type Identifier, present } from "@destack/schema";
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
import { CommandReference } from "@destack/object";
import { createMemo, For, type JSX, Show, useLocale } from "@destack/view";
import { ObjectReference } from "@destack/sync";
import {
    type ObjectEntry,
    Palette,
    type PaletteEntry,
    type PaletteFocus,
    type ViewEntry,
    type WindowEntry,
} from "@destack/view/palette";
import type { PaletteListing } from "./listing.ts";
import type { PaletteRun, RunEntry } from "./run.ts";

/** Search the palette's entries and act on the one the person picks. */
export function SearchStep(properties: {
    readonly listing: PaletteListing;
    readonly focus: PaletteFocus;
    readonly running: PaletteRun;
    readonly onOpen: (entry: ViewEntry | ObjectEntry) => void;
    readonly onWindow: (entry: WindowEntry) => void;
}): JSX.Element {
    const { running } = properties;

    return (
        <Command shouldFilter={false}>
            <PaletteSearch
                listing={properties.listing}
                focus={properties.focus}
                scope={running.scope()}
                onCommand={(command) => running.run(command)}
                onOpen={properties.onOpen}
                onWindow={properties.onWindow}
                onSpace={(space) => running.narrow(space)}
            />
        </Command>
    );
}

/** The choices a palette's search hands on. */
interface SearchChoices {
    /** Run a command of an installation, or of the page under the palette. */
    readonly onCommand: (command: RunEntry) => void;
    /** Open a view, or an object in the view presenting it. */
    readonly onOpen: (entry: ViewEntry | ObjectEntry) => void;
    /** Bring a window of this computer into focus. */
    readonly onWindow: (entry: WindowEntry) => void;
    /** Narrow the search to a space. */
    readonly onSpace: (space: Identifier<"space">) => void;
}

/** Render the ranked entries matching the typed text, and the spaces the palette could not read. */
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
            {/* Search */}
            <CommandInput placeholder={locale.render(t`Run a command or open a view`)} />

            {/* Narrowed space */}
            <Show when={properties.scope}>
                {(scope) => (
                    <div data-slot="command-scope">
                        {present(spaces().get(scope()), "the narrowed space's entry")}
                    </div>
                )}
            </Show>

            {/* Entries */}
            <CommandList>
                <CommandEmpty>{locale.render(t`No commands`)}</CommandEmpty>
                <EntryGroups {...properties} ranked={ranked()} spaces={spaces()} />
            </CommandList>

            {/* Unread spaces */}
            <Show when={properties.listing.failures.length > 0}>
                <ul data-slot="command-failures">
                    <For each={properties.listing.failures}>{(failure) => <li>{failure}</li>}</For>
                </ul>
            </Show>
        </>
    );
}

/** Render the ranked entries in one group per kind. */
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
                heading={locale.render(t`Windows`)}
                entries={kind("window")}
                spaces={properties.spaces}
                onSelect={properties.onWindow}
            />
            <EntryGroup
                heading={locale.render(t`Commands`)}
                entries={[...kind("registered"), ...kind("command")]}
                spaces={properties.spaces}
                onSelect={properties.onCommand}
            />
            <EntryGroup
                heading={locale.render(t`Views`)}
                entries={kind("view")}
                spaces={properties.spaces}
                onSelect={properties.onOpen}
            />
            <EntryGroup
                heading={locale.render(t`Spaces`)}
                entries={kind("space")}
                spaces={properties.spaces}
                onSelect={(entry) => properties.onSpace(entry.id)}
            />
            <EntryGroup
                heading={locale.render(t`Recent`)}
                entries={kind("object")}
                spaces={properties.spaces}
                onSelect={properties.onOpen}
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
                        <Show
                            when={
                                entry.kind === "command" || entry.kind === "registered"
                                    ? entry.category
                                    : null
                            }
                        >
                            {(category) => <span data-slot="command-category">{category()}: </span>}
                        </Show>
                        {entry.title}
                        <Show when={"space" in entry ? properties.spaces.get(entry.space) : null}>
                            {(space) => <span data-slot="command-space">{space()}</span>}
                        </Show>
                        <Show
                            when={
                                entry.kind === "command" || entry.kind === "registered"
                                    ? entry.keybinding
                                    : null
                            }
                        >
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
    // name a command by its reference
    if (entry.kind === "command") {
        return `${entry.space}/${CommandReference.format(entry)}`;
    }
    // name a command of the view under the palette by its package and name
    else if (entry.kind === "registered") {
        return `page/${entry.packageId}/${entry.name}`;
    }
    // name a view by its installation
    else if (entry.kind === "view") {
        return `${entry.space}/${entry.installation}/${entry.name}`;
    }
    // name an object by its key
    else if (entry.kind === "object") {
        return `${entry.space}/${ObjectReference.key(entry.object)}`;
    }
    // name any other entry by its identifier
    else {
        return entry.id;
    }
}
