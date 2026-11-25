import { Search, type SearchRange } from "@destack/db";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { swatch, weight } from "@destack/theme/tokens.stylex";
import { createMemo, For, type JSX, omit, Show, useContext } from "@destack/view";
import { AutocompleteContext } from "../autocomplete/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a highlighted text. */
const styles = style.create({
    mark: {
        backgroundColor: "transparent",
        color: { default: swatch.accentText, [media.forcedColors]: "Highlight" },
        fontWeight: weight.semibold,
    },
});

/** The properties of a highlighted text, the native element's attributes included. */
export type HighlightProperties = Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class" | "style" | "children"
> &
    ElementPartProperties & {
        /** The text. */
        readonly text: string;
        /** The search it marks, the nearest list's by default. */
        readonly search?: Search | undefined;
    };

/** A run of a text, matched or not. */
interface Run {
    /** The text. */
    readonly text: string;
    /** Whether the search matches it. */
    readonly isMatch: boolean;
}

/** Render a text with what a search matches in it marked. */
export function Highlight(properties: HighlightProperties): JSX.Element {
    // read the places from the search around
    const autocomplete = useContext(AutocompleteContext);
    const rest = omit(properties, "text", "search");
    const runs = createMemo(() => {
        const search = properties.search ?? autocomplete?.search() ?? new Search([]);

        return runsOf(properties.text, search.ranges(properties.text));
    });

    return renderPart("span", "highlight", rest, null, {
        get children() {
            return (
                <For each={runs()} keyed={false}>
                    {(run) => (
                        <Show when={run().isMatch} fallback={run().text}>
                            <mark {...style.attrs(styles.mark)}>{run().text}</mark>
                        </Show>
                    )}
                </For>
            );
        },
    });
}

/** Split a text into its runs, those in a place matched. */
function runsOf(text: string, ranges: readonly SearchRange[]): Run[] {
    // take each place, and what is before it
    const runs: Run[] = [];
    let last = 0;
    for (const [start, end] of ranges) {
        if (start > last) {
            runs.push({ text: text.slice(last, start), isMatch: false });
        }
        runs.push({ text: text.slice(start, end), isMatch: true });
        last = end;
    }

    return last < text.length ? [...runs, { text: text.slice(last), isMatch: false }] : runs;
}
