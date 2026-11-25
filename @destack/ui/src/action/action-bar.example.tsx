import { t } from "@destack/locale";
import { defineAction } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createSignal, For, handleOnPage, type JSX } from "@destack/view";
import { ActionButton, items, removeItems } from "./index.ts";
import { Checkbox } from "../checkbox/index.ts";
import { Label } from "../label/index.ts";
import { Stack } from "../stack/index.ts";
import { ActionBar, ActionBarClear, ActionBarCount, ActionBarMenu } from "./action-bar.tsx";

/** The styles of the notes the bar floats over. */
const styles = style.create({
    notes: {
        position: "relative",
        width: "100%",
        maxWidth: "36rem",
        minHeight: "14rem",
        marginInline: "auto",
        padding: space[4],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
    },
});

/** The notes a person selects. */
const NOTES = ["Launch plan", "Price list", "Press note"] as const;

/** Archive the selected notes. */
const archive = defineAction({ name: "notes.archive", title: t`Archive`, input: { items } });

/** Rename the selected notes. */
const rename = defineAction({ name: "notes.rename", title: t`Rename`, input: { items } });

/** Render notes a person selects, with the actions on them over their end. */
function Notes(): JSX.Element {
    // select the first, and leave out those archived or deleted
    const [notes, setNotes] = createSignal<readonly string[]>(NOTES);
    const [selected, setSelected] = createSignal<readonly string[]>(["Launch plan"]);
    const drop = (keys: readonly string[]) => {
        setNotes((all) => all.filter((note) => !keys.includes(note)));
        setSelected([]);
    };
    handleOnPage(archive, { run: ({ values }) => drop(values.items) });
    handleOnPage(removeItems, { run: ({ values }) => drop(values.items) });
    handleOnPage(rename, {
        run: ({ values }) =>
            void setNotes((all) =>
                all.map((note) => (values.items.includes(note) ? `${note} (new)` : note)),
            ),
    });

    return (
        <div {...style.attrs(styles.notes)}>
            <Stack space="3">
                <For each={notes()}>
                    {(note) => (
                        <Label>
                            <Checkbox
                                checked={selected().includes(note)}
                                onCheckedChange={(isOn) =>
                                    setSelected((all) =>
                                        isOn ? [...all, note] : all.filter((each) => each !== note),
                                    )
                                }
                            />
                            {note}
                        </Label>
                    )}
                </For>
            </Stack>
            <ActionBar items={selected()} onClear={() => setSelected([])}>
                <ActionBarCount />
                <ActionButton action={archive} variant="ghost" size="sm" />
                <ActionBarMenu />
                <ActionBarClear />
            </ActionBar>
        </div>
    );
}

/** The actions on selected notes, over their end while any are selected. */
export const actionBarNotes = defineExample({
    of: ActionBar,
    name: "notes",
    description: "the actions on selected notes, over their end while any are selected",
    layout: "padded",
    render: () => <Notes />,
});
