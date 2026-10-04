import * as style from "@destack/style";
import { createTheme } from "@destack/theme";
import { color } from "@destack/theme/tokens.stylex";
import "@destack/theme/theme.css";
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
import type { Identifier } from "@destack/schema";
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
    return (
        <main
            {...style.attrs(styles.page)}
            {...createTheme({ gray: "sand", accent: "orange", appearance: "system" })}
        >
            <Loading>
                <Workspace />
            </Loading>
        </main>
    );
}

/** List notebooks and notes, and edit the open note. */
function Workspace() {
    // follow the notebooks, remounting the notes whenever another notebook is chosen
    const [open, setOpen] = createSignal<Identifier<"note">>();
    const [chosen, setChosen] = createSignal<Identifier<"notebook">>();
    const space = useSpace({ notebook, note });
    const notebooks = useQuery(space.query.notebook.findMany({ orderBy: { name: "asc" } }));
    const selection = createMemo(() => ({ notebook: chosen() }));

    /** Undo and redo with the platform's shortcuts. */
    function shortcut(event: KeyboardEvent) {
        if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
            event.preventDefault();
            void (event.shiftKey ? space.redo() : space.undo());
        }
    }

    /** Create a notebook named by the form. */
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
            const created = await space.mutate.notebook.create({ name }).predicted;
            setChosen(created.id);
        }
    }

    /** Create an empty note in the chosen notebook and open it. */
    async function createNote() {
        const created = await space.mutate.note.create({
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
                <form onSubmit={(event) => void createNotebook(event)}>
                    <input name="name" aria-label="Notebook name" placeholder="New notebook" />
                </form>
            </nav>

            {/* Notes of the chosen notebook */}
            <section {...style.attrs(styles.column)} aria-label="Notes">
                <button onClick={() => void createNote()}>New note</button>
                <Show when={selection()} keyed>
                    {(selected) => <Notebook notebook={selected.notebook} open={setOpen} />}
                </Show>
            </section>

            {/* The open note */}
            <Show when={open()}>{(id) => <Editor id={id()} />}</Show>
        </div>
    );
}

/** List the notes of a notebook, or the loose notes, opening the one clicked. */
function Notebook(properties: {
    /** The notebook, absent for the loose notes. */
    notebook: Identifier<"notebook"> | undefined;
    /** Open a note. */
    open: (id: Identifier<"note">) => void;
}) {
    // follow the notebook's notes
    const space = useSpace({ note });
    const notes = useQuery(() =>
        space.query.note.findMany({
            where:
                properties.notebook === undefined
                    ? { parentId: { isNull: true } }
                    : { parentId: properties.notebook },
            orderBy: { title: "asc" },
        }),
    );

    return (
        <For each={notes()}>
            {(row) => <button onClick={() => properties.open(row.id)}>{row.title}</button>}
        </For>
    );
}

/** Edit one note's title and text, sharing every keystroke live. */
function Editor(properties: {
    /** The open note. */
    id: Identifier<"note">;
}) {
    // follow the note's row and its text
    const space = useSpace({ note });
    const current = useQuery(() => space.query.note.findFirst({ where: { id: properties.id } }));
    const body = useText(note, properties.id, "body");

    return (
        <article {...style.attrs(styles.column)} aria-label="Note">
            <input
                aria-label="Title"
                value={current()?.title ?? ""}
                onChange={(event) =>
                    void space.mutate.note.update({
                        id: properties.id,
                        title: event.currentTarget.value,
                    })
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
