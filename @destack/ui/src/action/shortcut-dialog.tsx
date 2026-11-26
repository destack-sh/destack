import { t } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { createSignal, For, handleOnPage, type JSX, useActions, useLocale } from "@destack/view";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../dialog/index.ts";
import { KbdShortcut, useCommandPlatform } from "../kbd/index.ts";
import { SearchField } from "../search-field/index.ts";
import { showShortcuts } from "./action.ts";

/** The styles of a dialog of shortcuts. */
const styles = style.create({
    list: {
        maxHeight: "60vh",
        overflow: "auto",
        overscrollBehavior: "contain",
    },
    heading: {
        margin: 0,
        marginBlockStart: space[4],
        marginBlockEnd: space[1],
        fontWeight: weight.semibold,
    },
    row: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[4],
        paddingBlock: space[1],
    },
    note: {
        color: color.mutedForeground,
    },
});

/** An action's title and keys. */
interface ShortcutRow {
    /** The title. */
    readonly title: string;
    /** The keys. */
    readonly keys: string;
}

/** Render the dialog of what the keys do where a person is, which a search narrows. */
export function ShortcutDialog(): JSX.Element {
    // open over where the person is
    const actions = useActions();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const [place, setPlace] = createSignal<Element | null>(null, { ownedWrite: true });
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const [words, setWords] = createSignal("", { ownedWrite: true });
    handleOnPage(showShortcuts, {
        run: ({ element }) => {
            setPlace(element);
            setWords("");
            setOpen(true);
        },
    });

    // list the actions there by category, those whose title has the words
    const groups = (): readonly (readonly [string, readonly ShortcutRow[]])[] => {
        // list those with keys
        const sought = words().trim().toLowerCase();
        const rows = (isOpen() ? actions.actions(place()) : []).flatMap((action) => {
            // take its first keys
            const keys = actions.keybindings(action, isCommand())[0];
            const title = locale.render(action.title);
            const category = locale.render(action.category ?? t.context("title")`Other`);

            return keys === undefined || !title.toLowerCase().includes(sought)
                ? []
                : [{ title, keys, category }];
        });

        // put those of no category last
        const other = locale.render(t.context("title")`Other`);
        const grouped = [...Map.groupBy(rows, ({ category }) => category)];

        return grouped.toSorted(([one], [two]) => Number(one === other) - Number(two === other));
    };

    return (
        <Dialog open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{locale.render(showShortcuts.title)}</DialogTitle>
                </DialogHeader>
                <SearchField
                    aria-label={locale.render(t.context("label")`Search shortcuts`)}
                    placeholder={locale.render(t.context("label")`Search shortcuts`)}
                    value={words()}
                    onValueChange={(next) => void setWords(next)}
                />
                <div {...style.attrs(text.sm, styles.list)}>
                    <For
                        each={groups()}
                        fallback={
                            <span {...style.attrs(styles.note)}>
                                {locale.render(t.context("title")`No results.`)}
                            </span>
                        }
                    >
                        {([category, rows]) => (
                            <section>
                                <h3 {...style.attrs(text.sm, styles.heading)}>{category}</h3>
                                <For each={rows}>
                                    {(row) => (
                                        <div {...style.attrs(styles.row)}>
                                            {row.title}
                                            <KbdShortcut keys={row.keys} />
                                        </div>
                                    )}
                                </For>
                            </section>
                        )}
                    </For>
                </div>
            </DialogContent>
        </Dialog>
    );
}
