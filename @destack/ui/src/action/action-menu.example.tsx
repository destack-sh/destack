import { t } from "@destack/locale";
import { defineAction, field } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { createSignal, handleOnPage, type JSX, omit, useActions } from "@destack/view";
import {
    ActionDialog,
    duplicateItems,
    items,
    openItem,
    removeItems,
    ShortcutDialog,
    ShortcutHint,
} from "./index.ts";
import { ActionBar } from "./action-bar.tsx";
import {
    GridList,
    GridListCell,
    GridListControl,
    GridListProvider,
    GridListViewport,
} from "../grid-list/index.ts";
import { ActionPalette } from "./action-palette.tsx";
import { Stack } from "../stack/index.ts";
import { ActionMenu } from "./action-menu.tsx";

/** The styles of the notes. */
const styles = style.create({
    page: {
        position: "relative",
        width: "100%",
        maxWidth: "32rem",
        marginInline: "auto",
        minHeight: "20rem",
        padding: space[4],
    },
    row: {
        justifyContent: "flex-start",
        width: "100%",
        gap: space[4],
        padding: space[2],
    },
    title: {
        flexGrow: 1,
    },
    shelf: {
        color: color.mutedForeground,
    },
});

/** A note. */
interface Note {
    /** The id. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** The shelf it is on. */
    readonly shelf: string;
}

/** The notes. */
const NOTES: readonly Note[] = ["Lisbon", "Order form", "Margins", "Newsletter"].map(
    (title, index) => ({ id: String(index), title, shelf: "Desk" }),
);

/** Put notes on a shelf. */
const putOn = defineAction({
    name: "note.putOn",
    title: t`Put on shelf`,
    category: t.context("title")`Note`,
    keybinding: "s",
    input: { items, shelf: field.enum(["Desk", "Archive", "Press"]) },
});

/** Put notes in the archive, by two keys in a row. */
const archive = defineAction({
    name: "note.archive",
    title: t.context("action")`Archive`,
    category: t.context("title")`Note`,
    keybinding: "g a",
    input: { items },
});

/** Put notes on the desk, by two keys in a row. */
const take = defineAction({
    name: "note.take",
    title: t`Put on desk`,
    category: t.context("title")`Note`,
    keybinding: "g d",
    input: { items },
});

/** Render notes with the actions on them in menus, a bar, a palette and keys. */
function Notes(): JSX.Element {
    // select some, and change them
    const [notes, setNotes] = createSignal(NOTES);
    const actions = useActions();
    const [opened, setOpened] = createSignal("");
    const put = (ids: readonly string[], shelf: string): void =>
        void setNotes((all) =>
            all.map((note) => (ids.includes(note.id) ? { ...note, shelf } : note)),
        );
    const list = new GridListControl<Note>({
        sections: () => [{ key: "notes", items: notes() }],
        key: (note) => note.id,
        text: (note) => note.title,
        selection: { multiple: true, behavior: "replace" },
        onAction: (note) => void setOpened(note.title),
    });
    const selected = () => list.selection?.values() ?? [];
    handleOnPage(putOn, {
        chosen: (_input, element) => {
            // read the shelf the notes there share
            const on = items.read(actions.places(element)) ?? [];
            const shelves = notes().flatMap(({ id, shelf }) => (on.includes(id) ? [shelf] : []));

            return [...new Set(shelves)].slice(0, new Set(shelves).size === 1 ? 1 : 0);
        },
        run: ({ values }) => put(values.items, values.shelf),
    });
    handleOnPage(archive, { run: ({ values }) => put(values.items, "Archive") });
    handleOnPage(take, { run: ({ values }) => put(values.items, "Desk") });
    handleOnPage(openItem, {
        run: ({ values }) => {
            const found = notes().find(({ id }) => values.items.includes(id));
            setOpened(found?.title ?? "");
        },
    });
    handleOnPage(duplicateItems, {
        run: ({ values }) =>
            void setNotes((all) =>
                all.flatMap((note) =>
                    values.items.includes(note.id)
                        ? [note, { ...note, id: `${note.id} copy` }]
                        : [note],
                ),
            ),
    });
    handleOnPage(removeItems, {
        run: ({ values }) =>
            void setNotes((all) => all.filter(({ id }) => !values.items.includes(id))),
    });

    return (
        <div {...style.attrs(styles.page)}>
            <Stack>
                <GridListProvider control={list}>
                    <GridListViewport>
                        <GridList control={list} aria-label="Notes" label={(note) => note.title}>
                            {(cell) => (
                                <GridListCell
                                    {...omit(cell, "item")}
                                    xstyle={[text.sm, styles.row]}
                                >
                                    <span {...style.attrs(styles.title)}>{cell.item.title}</span>
                                    <span {...style.attrs(styles.shelf)}>{cell.item.shelf}</span>
                                </GridListCell>
                            )}
                        </GridList>
                    </GridListViewport>
                </GridListProvider>
                <output>{opened()}</output>
            </Stack>
            <ActionBar items={selected()} onClear={() => list.selection?.clear()} />
            <ActionMenu />
            <ActionDialog />
            <ActionPalette id="notes" title="Notes" />
            <ShortcutDialog />
            <ShortcutHint />
        </div>
    );
}

/** Notes whose actions show in menus, a bar, a palette, a list of keys and a hint. */
export const actionMenuNotes = defineExample({
    of: ActionMenu,
    name: "notes",
    layout: "padded",
    description: t`Notes whose actions show in a menu by right click, a bar, a palette, a list of keys and a hint.`,
    render: () => <Notes />,
});
