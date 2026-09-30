import * as style from "@destack/style";
import { createTheme } from "@destack/theme";
import { color } from "@destack/theme/tokens.stylex";
import "@destack/theme/theme.css";
import { Condition } from "@destack/db/query";
import type { ObjectClient } from "@destack/object/client";
import {
    createMemo,
    createSignal,
    For,
    Loading,
    Show,
    useQuery,
    useText,
    useView,
} from "@destack/view";
import { note, notebook } from "../object/index.ts";

/** The notes layout: notebooks, their notes, and the open note. */
const styles = style.create({
    page: {
        display: "grid",
        gridTemplateColumns: "14rem 16rem 1fr",
        minHeight: "100vh",
        backgroundColor: color.background,
        color: color.foreground,
        fontFamily: "system-ui",
    },
    workspace: { display: "contents" },
    column: { display: "grid", alignContent: "start", gap: "0.5rem", padding: "1rem" },
    body: { minHeight: "60vh", font: "inherit", resize: "vertical" },
});

/** Show the space's notebooks and notes, editing the open note together. */
export default function Notes() {
    // open the space's notebooks and notes
    const view = useView();
    const objects = createMemo(() => view.client(view.space, [notebook, note]));

    return (
        <main
            {...style.attrs(styles.page)}
            {...createTheme({ gray: "sand", accent: "orange", appearance: "system" })}
        >
            <Loading>
                <Show when={objects()}>{(client) => <Workspace objects={client()} />}</Show>
            </Loading>
        </main>
    );
}

/** List notebooks and notes, and edit the open note. */
function Workspace(properties: {
    /** The space's objects. */
    objects: ObjectClient;
}) {
    // follow the notebooks, remounting the notes whenever another notebook is chosen
    const [open, setOpen] = createSignal<string>();
    const [chosen, setChosen] = createSignal<string>();
    const notebooks = useQuery(properties.objects, notebook, {
        order: [{ column: "name", direction: "asc" }],
    });
    const selection = createMemo(() => ({ notebook: chosen() }));

    /** Undo and redo with the platform's shortcuts. */
    function shortcut(event: KeyboardEvent) {
        if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
            event.preventDefault();
            void (event.shiftKey ? properties.objects.redo() : properties.objects.undo());
        }
    }

    /** Create a notebook named by the form. */
    async function createNotebook(event: SubmitEvent) {
        // read the entered name
        event.preventDefault();
        const form = event.currentTarget as HTMLFormElement;
        const name = new FormData(form).get("name");

        // create the notebook and choose it
        if (typeof name === "string" && name.trim() !== "") {
            form.reset();
            const created = await properties.objects.mutate(notebook).create({ name }).predicted;
            setChosen(created.id);
        }
    }

    /** Create an empty note in the chosen notebook and open it. */
    async function createNote() {
        const created = await properties.objects.mutate(note).create({
            title: "Untitled",
            ...(chosen() === undefined ? {} : { parentId: chosen() }),
        }).predicted;
        setOpen(created.id);
    }

    return (
        <div {...style.attrs(styles.workspace)} onKeyDown={shortcut}>
            {/* Notebooks */}
            <nav {...style.attrs(styles.column)} aria-label="Notebooks">
                <button onClick={() => setChosen(undefined)}>All loose notes</button>
                <For each={notebooks()}>
                    {(row) => <button onClick={() => setChosen(row.id)}>{row.name}</button>}
                </For>
                <form onSubmit={createNotebook}>
                    <input name="name" aria-label="Notebook name" placeholder="New notebook" />
                </form>
            </nav>

            {/* Notes of the chosen notebook */}
            <section {...style.attrs(styles.column)} aria-label="Notes">
                <button onClick={() => void createNote()}>New note</button>
                <Show when={selection()} keyed>
                    {(selected) => (
                        <Notebook
                            objects={properties.objects}
                            notebook={selected.notebook}
                            open={setOpen}
                        />
                    )}
                </Show>
            </section>

            {/* The open note */}
            <Show when={open()}>{(id) => <Editor objects={properties.objects} id={id()} />}</Show>
        </div>
    );
}

/** List the notes of a notebook, or the loose notes, opening the one clicked. */
function Notebook(properties: {
    /** The space's objects. */
    objects: ObjectClient;
    /** The notebook, absent for the loose notes. */
    notebook: string | undefined;
    /** Open a note. */
    open: (id: string) => void;
}) {
    // follow the notebook's notes
    const notes = useQuery(properties.objects, note, {
        where:
            properties.notebook === undefined
                ? Condition.missing("parentId")
                : Condition.eq("parentId", properties.notebook),
        order: [{ column: "title", direction: "asc" }],
    });

    return (
        <For each={notes()}>
            {(row) => <button onClick={() => properties.open(row.id)}>{row.title}</button>}
        </For>
    );
}

/** Edit one note's title and text, sharing every keystroke live. */
function Editor(properties: {
    /** The space's objects. */
    objects: ObjectClient;
    /** The open note. */
    id: string;
}) {
    // follow the note's row and its text
    const rows = useQuery(properties.objects, note, {
        where: Condition.eq("id", properties.id),
    });
    const body = useText(properties.objects, note, properties.id, "body");

    return (
        <article {...style.attrs(styles.column)} aria-label="Note">
            <input
                aria-label="Title"
                value={rows()[0]?.title ?? ""}
                onChange={(event) =>
                    void properties.objects
                        .mutate(note)
                        .update({ id: properties.id, title: event.currentTarget.value })
                }
            />
            <textarea
                {...style.attrs(styles.body)}
                aria-label="Text"
                value={body.text()}
                onInput={(event) => void body.replace(event.currentTarget.value)}
            />
        </article>
    );
}
