import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { space } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/icon";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
} from "@destack/ui/empty";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
} from "@destack/ui/input-group";
import { ItemContent, ItemTitle, itemStyle } from "@destack/ui/item";
import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarGroup,
    SidebarGroupContent,
    SidebarGroupLabel,
    SidebarHeader,
    SidebarInset,
    SidebarMenu,
    SidebarMenuBadge,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarProvider,
    SidebarTrigger,
} from "@destack/ui/sidebar";
import { Textarea } from "@destack/ui/textarea";
import {
    createMemo,
    createSignal,
    For,
    Loading,
    Show,
    useQuery,
    useSpace,
    useText,
} from "@destack/view";
import { Field } from "@destack/view/form";
import type { Identifier } from "@destack/schema";
import { note, notebook } from "../object/index.ts";

/** The notes layout: the note list beside the open note. */
const styles = style.create({
    inset: {
        display: "grid",
        gridTemplateRows: "auto 1fr",
        minHeight: "100vh",
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingInline: space[4],
        paddingBlock: space[2],
    },
    workspace: {
        display: "grid",
        gridTemplateColumns: "18rem 1fr",
        minHeight: 0,
    },
    list: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        padding: space[3],
        overflowY: "auto",
    },
    note: {
        width: "100%",
        textAlign: "start",
        cursor: "pointer",
    },
    editor: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
        padding: space[5],
    },
    body: {
        flex: 1,
        minHeight: "60vh",
        resize: "vertical",
    },
});

/** Show the space's notebooks and notes, editing the open note together. */
export default function Notes() {
    return (
        <Loading>
            <Workspace />
        </Loading>
    );
}

/** List notebooks and notes, and edit the open note. */
function Workspace() {
    // follow the open note and the chosen notebook to remount the notes on each choice
    const [open, setOpen] = createSignal<Identifier<"note">>();
    const [chosen, setChosen] = createSignal<Identifier<"notebook">>();
    const objects = useSpace({ notebook, note });
    const selection = createMemo(() => ({ notebook: chosen() }));

    /** Undo and redo with the platform's shortcuts. */
    function shortcut(event: KeyboardEvent) {
        if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
            event.preventDefault();
            void (event.shiftKey ? objects.redo() : objects.undo());
        }
    }

    /** Create an empty note in the chosen notebook and open it. */
    async function createNote() {
        const created = await objects.mutate.note.create({
            title: "Untitled",
            ...(chosen() === undefined ? {} : { parentId: chosen() }),
        }).predicted;
        setOpen(created.id);
    }

    return (
        <SidebarProvider onKeyDown={shortcut}>
            <Notebooks chosen={chosen()} choose={setChosen} />
            <SidebarInset style={styles.inset}>
                <header {...style.attrs(styles.header)}>
                    <SidebarTrigger />
                </header>
                <div {...style.attrs(styles.workspace)}>
                    <Show when={selection()} keyed>
                        {(selected) => (
                            <NoteList
                                notebook={selected.notebook}
                                open={open()}
                                choose={setOpen}
                                create={() => void createNote()}
                            />
                        )}
                    </Show>
                    <Show when={open()}>{(id) => <Editor id={id()} />}</Show>
                </div>
            </SidebarInset>
        </SidebarProvider>
    );
}

/** List the space's notebooks and the loose notes to choose from, and create a notebook named by the form. */
function Notebooks(properties: {
    /** The chosen notebook, absent for the loose notes. */
    chosen: Identifier<"notebook"> | undefined;
    /** Choose a notebook, or the loose notes. */
    choose: (id: Identifier<"notebook"> | undefined) => void;
}) {
    // follow the notebooks by name
    const objects = useSpace({ notebook });
    const notebooks = useQuery(objects.query.notebook.findMany({ orderBy: { name: "asc" } }));

    /** Create a notebook named by the form and choose it. */
    async function createNotebook(
        event: SubmitEvent & { readonly currentTarget: HTMLFormElement },
    ) {
        // read the entered name
        event.preventDefault();
        const form = event.currentTarget;
        const name = new FormData(form).get("name");

        // create the notebook and choose it
        if (typeof name === "string" && name.trim() !== "") {
            form.reset();
            const created = await objects.mutate.notebook.create({ name }).predicted;
            properties.choose(created.id);
        }
    }

    return (
        <Sidebar collapsible="icon">
            <SidebarHeader>
                <span {...style.attrs(text.headline)}>Notes</span>
            </SidebarHeader>
            <SidebarContent>
                <SidebarGroup>
                    <SidebarGroupLabel>Notebooks</SidebarGroupLabel>
                    <SidebarGroupContent>
                        <SidebarMenu>
                            <SidebarMenuItem>
                                <SidebarMenuButton
                                    isActive={properties.chosen === undefined}
                                    tooltip="Loose notes"
                                    onClick={() => properties.choose(undefined)}
                                >
                                    <Icon name="note" />
                                    Loose notes
                                </SidebarMenuButton>
                            </SidebarMenuItem>
                            <For each={notebooks()}>
                                {(row) => (
                                    <SidebarMenuItem>
                                        <SidebarMenuButton
                                            isActive={properties.chosen === row.id}
                                            tooltip={row.name}
                                            onClick={() => properties.choose(row.id)}
                                        >
                                            <Icon name="notebook" />
                                            {row.name}
                                        </SidebarMenuButton>
                                        <SidebarMenuBadge>{row.noteCount}</SidebarMenuBadge>
                                    </SidebarMenuItem>
                                )}
                            </For>
                        </SidebarMenu>
                    </SidebarGroupContent>
                </SidebarGroup>
            </SidebarContent>
            <SidebarFooter>
                <form onSubmit={(event) => void createNotebook(event)}>
                    <InputGroup>
                        <InputGroupInput
                            name="name"
                            aria-label="Notebook name"
                            placeholder="New notebook"
                        />
                        <InputGroupAddon align="inline-end">
                            <InputGroupButton
                                type="submit"
                                size="icon-xs"
                                aria-label="Create notebook"
                            >
                                <Icon name="plus" />
                            </InputGroupButton>
                        </InputGroupAddon>
                    </InputGroup>
                </form>
            </SidebarFooter>
        </Sidebar>
    );
}

/** List the notes of a notebook, or the loose notes, opening the one chosen. */
function NoteList(properties: {
    /** The notebook, absent for the loose notes. */
    notebook: Identifier<"notebook"> | undefined;
    /** The open note. */
    open: Identifier<"note"> | undefined;
    /** Open a note. */
    choose: (id: Identifier<"note">) => void;
    /** Create a note and open it. */
    create: () => void;
}) {
    // follow the notebook's notes with pinned ones first
    const objects = useSpace({ note });
    const notes = useQuery(() =>
        objects.query.note.findMany({
            where:
                properties.notebook === undefined
                    ? { parentId: { isNull: true } }
                    : { parentId: properties.notebook },
            orderBy: { pinned: "desc", title: "asc" },
        }),
    );

    return (
        <section {...style.attrs(styles.list)} aria-label="Notes">
            <Button variant="outline" onClick={() => properties.create()}>
                <Icon name="plus" />
                New note
            </Button>
            <For
                each={notes()}
                fallback={
                    <Empty>
                        <EmptyHeader>
                            <EmptyMedia variant="icon">
                                <Icon name="note" />
                            </EmptyMedia>
                            <EmptyTitle>No notes yet</EmptyTitle>
                            <EmptyDescription>
                                Notes you write here stay in this notebook.
                            </EmptyDescription>
                        </EmptyHeader>
                        <EmptyContent>
                            <Button onClick={() => properties.create()}>New note</Button>
                        </EmptyContent>
                    </Empty>
                }
            >
                {(row) => (
                    <button
                        type="button"
                        aria-current={properties.open === row.id ? "true" : undefined}
                        onClick={() => properties.choose(row.id)}
                        {...style.attrs(
                            itemStyle({
                                variant: properties.open === row.id ? "muted" : "default",
                                size: "sm",
                            }),
                            styles.note,
                        )}
                    >
                        <ItemContent>
                            <ItemTitle>
                                {row.title}
                                <Show when={row.pinned}>
                                    <Badge variant="secondary">Pinned</Badge>
                                </Show>
                            </ItemTitle>
                        </ItemContent>
                    </button>
                )}
            </For>
        </section>
    );
}

/** Edit one note's title, pin and text, sharing every keystroke live. */
function Editor(properties: {
    /** The open note. */
    id: Identifier<"note">;
}) {
    // follow the note's row and its text
    const objects = useSpace({ note });
    const current = useQuery(() => objects.query.note.findFirst({ where: { id: properties.id } }));
    const body = useText(note, properties.id, "body");

    return (
        <Show when={current()}>
            {(row) => (
                <article {...style.attrs(styles.editor)} aria-label="Note">
                    <Field
                        label="Title"
                        for={{
                            object: note,
                            field: "title",
                            value: row().title,
                            write: (title) =>
                                objects.mutate.note.update({ id: properties.id, title }),
                        }}
                    />
                    <Field
                        label="Pinned"
                        orientation="horizontal"
                        for={{
                            object: note,
                            field: "pinned",
                            value: row().pinned,
                            write: (pinned) =>
                                objects.mutate.note.update({ id: properties.id, pinned }),
                        }}
                    />
                    <Textarea
                        aria-label="Text"
                        value={body.text()}
                        onInput={(event) => void body.replace(event.currentTarget.value)}
                        style={styles.body}
                    />
                </article>
            )}
        </Show>
    );
}
