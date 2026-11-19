/** A key press a view answers: moving, folding, opening details and selecting. */
export type ViewPress = "up" | "down" | "left" | "right" | "enter" | "select";

/** One line of a view: its object's key and its depth in the tree. */
export interface ViewLine {
    /** The key of the line's object, which the cursor, folds and details follow across updates. */
    readonly key: string;
    /** The object's depth in the tree, zero for a root. */
    readonly depth: number;
}

/** Where a person stands in a view: the cursor, folds, details and selection. */
export interface ObjectViewState {
    /** The key of the object under the cursor, the first line's when absent. */
    readonly cursor: string | undefined;
    /** The index of the first visible line shown. */
    readonly top: number;
    /** The keys of the objects whose subtrees are hidden. */
    readonly folded: ReadonlySet<string>;
    /** The key of the object whose details show under it, none when absent. */
    readonly opened: string | undefined;
    /** The keys of the selected objects, which bulk actions apply to. */
    readonly selected: ReadonlySet<string>;
}

/** A cursor over a view's nested lines that moves, folds, opens, selects and scrolls. */
export const ObjectViewState = {
    /** Start at the first line with nothing folded, opened or selected. */
    start(): ObjectViewState {
        return {
            cursor: undefined,
            top: 0,
            folded: new Set(),
            opened: undefined,
            selected: new Set(),
        };
    },

    /** List the lines a fold leaves visible: every line but the descendants of folded ones. */
    visible<Line extends ViewLine>(lines: readonly Line[], folded: ReadonlySet<string>): Line[] {
        // skip the lines deeper than a folded line until one at its depth or above
        const shown: Line[] = [];
        let hiddenBelow: number | undefined;
        for (const line of lines) {
            // show a line outside folded subtrees
            const isHidden = hiddenBelow !== undefined && line.depth > hiddenBelow;
            if (!isHidden) {
                hiddenBelow = folded.has(line.key) ? line.depth : undefined;
                shown.push(line);
            }
        }

        return shown;
    },

    /** Answer a key: move the cursor, fold or unfold its subtree, or open its details. */
    press(
        state: ObjectViewState,
        key: ViewPress,
        lines: readonly ViewLine[],
        height: number,
    ): ObjectViewState {
        // find the cursor among the visible lines, the first one when it is gone
        const shown = ObjectViewState.visible(lines, state.folded);
        const found = shown.findIndex((line) => line.key === state.cursor);
        const index = Math.max(0, found);
        const line = shown[index];
        if (line === undefined) {
            return state;
        }
        const hasChildren = (lines[lines.indexOf(line) + 1]?.depth ?? -1) > line.depth;
        const isFolded = state.folded.has(line.key);

        // move, fold, unfold or open by the key
        let next = index;
        let folded = state.folded;
        let opened = state.opened;
        let selected = state.selected;
        if (key === "up") {
            next = Math.max(0, index - 1);
        } else if (key === "down") {
            next = Math.min(shown.length - 1, index + 1);
        }
        // fold an open subtree, else step out to the parent
        else if (key === "left") {
            if (hasChildren && !isFolded) {
                folded = new Set([...state.folded, line.key]);
            } else {
                const parent = shown.findLastIndex(
                    (each, position) => position < index && each.depth < line.depth,
                );
                next = parent === -1 ? index : parent;
            }
        }
        // unfold a folded subtree, else step into its first child
        else if (key === "right") {
            if (isFolded) {
                folded = new Set([...state.folded].filter((each) => each !== line.key));
            } else if (hasChildren) {
                next = Math.min(shown.length - 1, index + 1);
            }
        }
        // open the details under the cursor, or close them
        else if (key === "enter") {
            opened = state.opened === line.key ? undefined : line.key;
        }
        // select the object under the cursor, or deselect it
        else {
            selected = state.selected.has(line.key)
                ? new Set([...state.selected].filter((each) => each !== line.key))
                : new Set([...state.selected, line.key]);
        }

        // scroll so the cursor stays within the height
        const top = Math.min(next, Math.max(state.top, next - height + 1));

        return { cursor: shown[next]?.key, top, folded, opened, selected };
    },
};
