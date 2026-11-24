import { Patch } from "@destack/code";
import tsx from "@destack/code/grammar/tsx";
import typescript from "@destack/code/grammar/typescript";
import { defineExample } from "@destack/package/declare";
import { createSignal, Show } from "@destack/view";
import { Stack } from "../stack/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { Diff, type DiffLayout } from "./index.ts";

/** A pricing page's plans as they were. */
const BEFORE = `# Plans

Every plan starts with a trial.
The trial takes thirty days.
Nothing is charged before it ends.
You can leave at any time.

## Solo
8 euros a month, for one person.
Support by email.

## Team
20 euros a month, for five people.
Support by email and chat.
`;

/** A pricing page's plans as they are. */
const AFTER = `# Plans

Every plan starts with a trial.
The trial takes fourteen days.
Nothing is charged before it ends.
You can leave at any time.

## Solo
9 euros a month, for one person.
Support by email.

## Team
24 euros a month, for ten people.
Support by email and chat.
Invoices on request.
`;

/** A pricing page's changes, in one column or side by side. */
export const diffPlans = defineExample({
    of: Diff,
    name: "plans",
    description: "a pricing page's changes, in one column or side by side",
    layout: "padded",
    render: () => {
        const [layout, setLayout] = createSignal<DiffLayout>("unified");

        return (
            <Stack space="3">
                <ToggleGroup
                    aria-label="Layout"
                    disallowEmptySelection
                    value={layout()}
                    onValueChange={(next) => setLayout(next === "split" ? "split" : "unified")}
                >
                    <ToggleGroupItem value="unified">Unified</ToggleGroupItem>
                    <ToggleGroupItem value="split">Split</ToggleGroupItem>
                </ToggleGroup>
                <Diff
                    aria-label="Changes to the plans"
                    before={BEFORE}
                    after={AFTER}
                    layout={layout()}
                    context={1}
                />
            </Stack>
        );
    },
});

/** A price's calculation as it was. */
const PRICE_BEFORE = `/** The price of a plan for a number of people. */
export function priceOf(plan: Plan, people: number): number {
    // add up the people past those included
    const extra = Math.max(0, people - 5);

    return plan.price + extra * 4;
}
`;

/** A price's calculation as it is. */
const PRICE_AFTER = `/** The price of a plan for a number of people. */
export function priceOf(plan: Plan, people: number): number {
    // add up the people past those included
    const extra = Math.max(0, people - plan.people);
    const total = plan.price + extra * plan.extra;

    return plan.isYearly ? total * 10 : total;
}
`;

/** A function's changes in its language's colors. */
export const diffCode = defineExample({
    of: Diff,
    name: "code",
    description: "a function's changes in its language's colors",
    layout: "padded",
    render: () => (
        <Diff
            aria-label="Changes to priceOf"
            before={PRICE_BEFORE}
            after={PRICE_AFTER}
            grammar={typescript}
        />
    ),
});

/** A list of notes as it was. */
const NOTES_BEFORE = `import { t } from "@destack/locale";
import { createMemo, createSignal, For, type JSX } from "@destack/view";
import { note } from "../note/index.ts";
import { NoteCard } from "./card.tsx";

/** The notes a page shows at first. */
const PAGE = 20;

/** What a list of notes shows. */
interface NotesProperties {
    /** The space whose notes they are. */
    readonly space: string;
    /** Open a note. */
    readonly onOpen: (id: string) => void;
}

/** Count the words of a text. */
function wordsOf(text: string): number {
    return text.split(" ").length;
}

/** Render the notes of a space, newest first. */
export function Notes(properties: NotesProperties): JSX.Element {
    const [search, setSearch] = createSignal("");
    const notes = useObjects(note, { space: properties.space });
    const found = createMemo(() =>
        notes().filter((note) => note.title.includes(search())),
    );

    return (
        <section aria-label="Notes">
            <input value={search()} onInput={(event) => setSearch(event.target.value)} />
            <For each={found()}>
                {(note) => <NoteCard note={note} onClick={() => properties.onOpen(note.id)} />}
            </For>
        </section>
    );
}

/** Sort notes by when they changed. */
export function byChange(one: Note, other: Note): number {
    return other.changed - one.changed;
}

/** Sort notes by their title. */
export function byTitle(one: Note, other: Note): number {
    return one.title.localeCompare(other.title);
}

/** Sort notes by how long they are. */
export function byLength(one: Note, other: Note): number {
    return wordsOf(one.text) - wordsOf(other.text);
}
`;

/** A list of notes as it is. */
const NOTES_AFTER = NOTES_BEFORE.replace(
    'return text.split(" ").length;',
    "return text.trim().split(/\\s+/u).length;",
)
    .replace(
        "notes().filter((note) => note.title.includes(search())),",
        `notes()
            .filter((note) => note.title.toLowerCase().includes(search().toLowerCase()))
            .slice(0, PAGE),`,
    )
    .replace(
        '<section aria-label="Notes">',
        "<section aria-label={t`Notes in this space`.toString()}>",
    )
    .replace("return other.changed - one.changed;", "return other.updatedAt - one.updatedAt;")
    .replace(
        `
/** Sort notes by their title. */
export function byTitle(one: Note, other: Note): number {
    return one.title.localeCompare(other.title);
}
`,
        "",
    );

/** A file under review: select lines, comment on them, and accept or reject each change. */
export const diffReview = defineExample({
    of: Diff,
    name: "review",
    description: "a file under review, whose changes are accepted, rejected and commented on",
    layout: "padded",
    render: () => {
        // reject a change by putting its old lines back
        const [after, setAfter] = createSignal(NOTES_AFTER);
        const [comments, setComments] = createSignal<
            readonly { side: "before" | "after"; line: number }[]
        >([{ side: "after", line: 18 }]);
        const [layout, setLayout] = createSignal<DiffLayout>("unified");

        return (
            <Stack space="3">
                <ToggleGroup
                    aria-label="Layout"
                    disallowEmptySelection
                    value={layout()}
                    onValueChange={(next) => setLayout(next === "split" ? "split" : "unified")}
                >
                    <ToggleGroupItem value="unified">Unified</ToggleGroupItem>
                    <ToggleGroupItem value="split">Split</ToggleGroupItem>
                </ToggleGroup>
                <Diff
                    path="src/notes/notes.tsx"
                    before={NOTES_BEFORE}
                    after={after()}
                    grammar={tsx}
                    layout={layout()}
                    onReject={(change) => {
                        const old = NOTES_BEFORE.split("\n").slice(
                            change.before.start,
                            change.before.end,
                        );
                        setAfter((current) =>
                            current
                                .split("\n")
                                .toSpliced(
                                    change.after.start,
                                    change.after.end - change.after.start,
                                    ...old,
                                )
                                .join("\n"),
                        );
                    }}
                    onAccept={() => undefined}
                    onComment={(side, line) =>
                        setComments((current) => [...current, { side, line }])
                    }
                    annotations={comments().map(({ side, line }) => ({
                        side,
                        line,
                        content: () => <p>Should this count words in scripts without spaces?</p>,
                    }))}
                />
            </Stack>
        );
    },
});

/** A patch as a tool writes it, whose lines between the hunks are not known. */
const PATCH = `diff --git a/src/note/title.ts b/src/note/name.ts
similarity index 82%
rename from src/note/title.ts
rename to src/note/name.ts
--- a/src/note/title.ts
+++ b/src/note/name.ts
@@ -12,5 +12,5 @@ export class Note {
     /** Read its title. */
     get title(): string {
-        return this.#title;
+        return this.#title.trim();
     }
 
@@ -40,4 +40,5 @@ export function write(note: Note): string {
     const lines = [note.title];
+    lines.push("");
     lines.push(note.text);
     return lines.join(BREAK);
 }
`;

/** A moved file's patch, which has only the lines it changes. */
export const diffPatch = defineExample({
    of: Diff,
    name: "patch",
    description: "a moved file's patch, which has only the lines it changes",
    layout: "padded",
    render: () => {
        const [file] = Patch.read(PATCH).files;

        return <Show when={file}>{(read) => <Diff patch={read()} grammar={typescript} />}</Show>;
    },
});
