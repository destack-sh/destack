import { Shortcut } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { color, space, stroke, weight } from "@destack/theme/tokens.stylex";
import type { IconBodies } from "@destack/icon";
import calendarBlank from "@destack/icon/phosphor/calendar-blank";
import checkSquare from "@destack/icon/phosphor/check-square";
import gear from "@destack/icon/phosphor/gear";
import kanban from "@destack/icon/phosphor/kanban";
import note from "@destack/icon/phosphor/note";
import plus from "@destack/icon/phosphor/plus";
import { Icon } from "../icon/index.ts";
import { createSignal, For, type JSX, Show } from "@destack/view";
import {
    Command,
    CommandDialog,
    CommandBody,
    CommandDetail,
    CommandEmpty,
    CommandFooter,
    CommandGroup,
    CommandInput,
    CommandHint,
    CommandItem,
    CommandItemDescription,
    CommandItemMeta,
    CommandList,
    CommandLoading,
    CommandPage,
    CommandSeparator,
    CommandShortcut,
    useCommand,
} from "./command.tsx";

/** The people a task can be assigned to. */
const PEOPLE = ["Ada Lovelace", "Jonas Weber", "Mia Sørensen"] as const;

/** The statuses a task can move to. */
const STATUSES = ["Backlog", "Doing", "Review", "Done"] as const;

/** A command palette that opens with ⌘K and runs note actions. */
export const commandDialogNoteActions = defineExample({
    of: CommandDialog,
    name: "note-actions",
    description: "a command palette that opens with ⌘K and runs note actions",
    render: () => {
        const [isOpen, setOpen] = createSignal(false);

        return (
            <div
                onKeyDown={(event) => {
                    // open the palette on ⌘K, or Ctrl+K off Command platforms
                    if (Shortcut.matches("mod+k", event, Shortcut.isCommand(navigator.userAgent))) {
                        setOpen(true);
                    }
                }}
            >
                <CommandDialog open={isOpen()} onOpenChange={setOpen}>
                    <CommandInput placeholder="Type a command" />
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <CommandGroup heading="Notes">
                            <CommandItem>
                                New note
                                <CommandShortcut keys="mod+n" />
                            </CommandItem>
                            <CommandItem keywords={["trash"]}>Delete note</CommandItem>
                        </CommandGroup>
                        <CommandSeparator />
                        <CommandGroup heading="Settings">
                            <CommandItem>Appearance</CommandItem>
                        </CommandGroup>
                    </CommandList>
                </CommandDialog>
            </div>
        );
    },
});

/** A list of note and settings commands with an unavailable one, reporting the command it runs. */
export const commandNoteCommands = defineExample({
    of: Command,
    name: "note-commands",
    description:
        "a list of note and settings commands with an unavailable one, reporting the command it runs",
    properties: {
        "aria-label": "Commands",
    },
    render: (properties) => {
        const [chosen, setChosen] = createSignal("");

        return (
            <>
                <Command {...properties}>
                    <CommandInput placeholder="Type a command" />
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <CommandGroup heading="Notes">
                            <CommandItem onSelect={setChosen}>New note</CommandItem>
                            <CommandItem keywords={["trash"]} onSelect={setChosen}>
                                Delete note
                            </CommandItem>
                        </CommandGroup>
                        <CommandSeparator />
                        <CommandGroup heading="Settings">
                            <CommandItem disabled>Billing</CommandItem>
                            <CommandItem value="theme" onSelect={setChosen}>
                                Appearance
                            </CommandItem>
                        </CommandGroup>
                    </CommandList>
                </Command>
                <output>{chosen()}</output>
            </>
        );
    },
});

/** A command list whose owner sets the highlight, reporting the option the arrow keys ask for. */
export const commandControlledHighlight = defineExample({
    of: Command,
    name: "controlled-highlight",
    description:
        "a command list whose owner controls the highlight, reporting the option the arrow keys ask for",
    properties: { value: "Appearance" },
    render: (properties) => {
        const [requested, setRequested] = createSignal("");

        return (
            <>
                <Command {...properties} onValueChange={setRequested}>
                    <CommandInput aria-label="Command" />
                    <CommandList>
                        <CommandItem>New note</CommandItem>
                        <CommandItem>Appearance</CommandItem>
                    </CommandList>
                </Command>
                <output>{requested()}</output>
            </>
        );
    },
});

/** The note and settings commands whose search finds none, showing the empty note. */
export const commandNoteCommandsNoResults = defineExample({
    of: Command,
    name: "note-commands-no-results",
    description: "the note and settings commands whose search finds none, showing the empty note",
    properties: {
        "aria-label": "Commands",
    },
    render: (properties) => (
        <Command {...properties}>
            <CommandInput placeholder="Type a command" defaultValue="export" />
            <CommandList>
                <CommandEmpty>No results</CommandEmpty>
                <CommandGroup heading="Notes">
                    <CommandItem>New note</CommandItem>
                    <CommandItem>Delete note</CommandItem>
                </CommandGroup>
            </CommandList>
        </Command>
    ),
});

/** A command list whose owner loads the matching commands, showing their loading. */
export const commandPeopleLoading = defineExample({
    of: Command,
    name: "people-loading",
    description: "a command list whose owner loads the matching commands, showing their loading",
    properties: {
        "aria-label": "People",
        shouldFilter: false,
    },
    render: (properties) => (
        <Command {...properties}>
            <CommandInput placeholder="Find a person" defaultValue="ada" />
            <CommandList>
                <CommandLoading>Searching…</CommandLoading>
            </CommandList>
        </Command>
    ),
});

/** The command palette open over the page, its first command highlighted. */
export const commandDialogNoteActionsOpen = defineExample({
    of: CommandDialog,
    name: "note-actions-open",
    description: "the command palette open over the page, its first command highlighted",
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <CommandDialog {...properties}>
            <CommandInput placeholder="Type a command" />
            <CommandList>
                <CommandEmpty>No results</CommandEmpty>
                <CommandGroup heading="Notes">
                    <CommandItem>
                        New note
                        <CommandShortcut keys="mod+n" />
                    </CommandItem>
                    <CommandItem keywords={["trash"]}>Delete note</CommandItem>
                </CommandGroup>
                <CommandSeparator />
                <CommandGroup heading="Settings">
                    <CommandItem>Appearance</CommandItem>
                </CommandGroup>
            </CommandList>
        </CommandDialog>
    ),
});

/** The actions on a task: assign it and move it from pages, or archive it by its key. */
function TaskActions(properties: { readonly onRun: (summary: string) => void }): JSX.Element {
    // run a choice on a page, then go back to the root
    const command = useCommand();
    const choose = (summary: string) => {
        properties.onRun(summary);
        command.pop();
    };

    return (
        <>
            <CommandPage>
                <CommandGroup heading="Task">
                    <CommandItem
                        onSelect={() => command.push({ name: "assign", title: "Assign to" })}
                    >
                        Assign to…
                        <CommandShortcut keys="a" />
                    </CommandItem>
                    <CommandItem onSelect={() => command.push({ name: "status", title: "Status" })}>
                        Change status…
                        <CommandShortcut keys="s" />
                    </CommandItem>
                    <CommandItem onSelect={() => properties.onRun("Archived")}>
                        Archive
                        <CommandShortcut keys="mod+backspace" />
                    </CommandItem>
                </CommandGroup>
            </CommandPage>
            <CommandPage name="assign">
                <CommandGroup heading="People">
                    <For each={PEOPLE}>
                        {(person) => (
                            <CommandItem onSelect={() => choose(`Assigned to ${person}`)}>
                                {person}
                            </CommandItem>
                        )}
                    </For>
                </CommandGroup>
            </CommandPage>
            <CommandPage name="status">
                <CommandGroup heading="Statuses">
                    <For each={STATUSES}>
                        {(status) => (
                            <CommandItem onSelect={() => choose(`Moved to ${status}`)}>
                                {status}
                            </CommandItem>
                        )}
                    </For>
                </CommandGroup>
            </CommandPage>
        </>
    );
}

/** The actions on a task, assigning it and moving it on pages that Backspace leaves. */
export const commandTaskActions = defineExample({
    of: Command,
    name: "task-actions",
    description: "the actions on a task, assigning it and moving it on pages that Backspace leaves",
    properties: {
        "aria-label": "Task actions",
    },
    render: (properties) => {
        const [summary, setSummary] = createSignal("");

        return (
            <>
                <Command {...properties}>
                    <CommandInput placeholder="Type a command" />
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <TaskActions onRun={setSummary} />
                    </CommandList>
                </Command>
                <p>{summary()}</p>
            </>
        );
    },
});

/** An entry of the space's search: what it opens or runs, where it lives and what it says. */
interface SearchRow {
    /** The value the palette highlights it by. */
    readonly value: string;
    /** The icon. */
    readonly icon: IconBodies;
    /** The title. */
    readonly title: string;
    /** The line after the title. */
    readonly description: string;
    /** The space it lives in. */
    readonly space: string;
    /** The keys that run it, none for most entries. */
    readonly keys?: string;
}

/** The space's search by group: what the person opened lately, what they can do, and where. */
const SEARCH: readonly { readonly heading: string; readonly rows: readonly SearchRow[] }[] = [
    {
        heading: "Recent",
        rows: [
            {
                value: "lisbon",
                icon: note,
                title: "Trip to Lisbon",
                description: "Note",
                space: "Home",
            },
            {
                value: "launch",
                icon: kanban,
                title: "Launch",
                description: "Project",
                space: "Harbour Studio",
            },
        ],
    },
    {
        heading: "Actions",
        rows: [
            {
                value: "new-task",
                icon: plus,
                title: "New task",
                description: "Tasks",
                space: "Harbour Studio",
                keys: "c",
            },
            {
                value: "new-event",
                icon: calendarBlank,
                title: "New event",
                description: "Calendar",
                space: "Home",
                keys: "mod+e",
            },
        ],
    },
    {
        heading: "Views",
        rows: [
            {
                value: "my-tasks",
                icon: checkSquare,
                title: "My tasks",
                description: "Tasks",
                space: "Harbour Studio",
            },
            {
                value: "settings",
                icon: gear,
                title: "Settings",
                description: "Space",
                space: "Home",
                keys: "mod+,",
            },
        ],
    },
];

/** The width of the search's frame, room for the list beside its detail. */
const SEARCH_WIDTH = "44rem";

/** The height of the search's frame, about eight options tall. */
const SEARCH_HEIGHT = "24rem";

/** The styles of the search's frame and its detail. */
const searchStyles = style.create({
    frame: {
        width: SEARCH_WIDTH,
        height: SEARCH_HEIGHT,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    title: {
        margin: 0,
        fontWeight: weight.medium,
    },
    line: {
        marginBlock: space[1],
        color: color.mutedForeground,
    },
});

/** A space's search: groups, the highlighted entry's detail and keys. */
export const commandSearchDetail = defineExample({
    of: Command,
    name: "search-detail",
    description:
        "a space's search: recent, actions and views, the highlighted entry's detail and keys",
    properties: {
        "aria-label": "Search",
    },
    render: (properties) => {
        const rows = SEARCH.flatMap((group) => group.rows);

        return (
            <Command {...properties} xstyle={searchStyles.frame}>
                <CommandInput placeholder="Search or run a command" />
                <CommandBody>
                    <CommandList>
                        <CommandEmpty>No results</CommandEmpty>
                        <For each={SEARCH}>
                            {(group) => (
                                <CommandGroup heading={group.heading}>
                                    <For each={group.rows}>
                                        {(row) => (
                                            <CommandItem
                                                value={row.value}
                                                keywords={[row.title, row.space]}
                                            >
                                                <Icon icon={row.icon} />
                                                {row.title}
                                                <CommandItemDescription>
                                                    {row.description}
                                                </CommandItemDescription>
                                                <CommandItemMeta>
                                                    {row.space}
                                                    <Show when={row.keys}>
                                                        {(keys) => (
                                                            <CommandShortcut keys={keys()} />
                                                        )}
                                                    </Show>
                                                </CommandItemMeta>
                                            </CommandItem>
                                        )}
                                    </For>
                                </CommandGroup>
                            )}
                        </For>
                    </CommandList>
                    <CommandDetail>
                        {(value) => {
                            const row = rows.find((entry) => entry.value === value);

                            return (
                                <Show when={row}>
                                    {(shown) => (
                                        <>
                                            <h3 {...style.attrs(searchStyles.title)}>
                                                {shown().title}
                                            </h3>
                                            <p {...style.attrs(searchStyles.line)}>
                                                {shown().description} in {shown().space}
                                            </p>
                                        </>
                                    )}
                                </Show>
                            );
                        }}
                    </CommandDetail>
                </CommandBody>
                <CommandFooter>
                    <CommandHint keys="enter">Open</CommandHint>
                    <CommandHint keys="mod+enter">Open in new window</CommandHint>
                </CommandFooter>
            </Command>
        );
    },
});
