import type { Range, Token } from "@destack/code";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { radius, space, swatch } from "@destack/theme/tokens.stylex";
import { createMemo, type JSX, Show, useLocale } from "@destack/view";
import { Icon } from "@destack/ui/icon";
import warning from "@destack/icon/phosphor/prohibit";
import { t } from "@destack/locale";
import { CodeTokens } from "../code/index.ts";
import type { DiffControl } from "./control.ts";
import type { DiffLine, DiffSide } from "./model.ts";

/** The styles of changed words: a theme's text on its tint, edged by the mark. */
const styles = style.create({
    open: { marginInlineStart: space[2], color: swatch.destructiveText },
    added: {
        backgroundColor: swatch.successTint,
        borderRadius: radius[1],
        boxDecorationBreak: "clone",
        boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${swatch.successMark} 45%, transparent)`,
        color: swatch.successTintText,
        textDecorationLine: { default: null, [media.forcedColors]: "underline" },
    },
    removed: {
        backgroundColor: swatch.destructiveTint,
        borderRadius: radius[1],
        boxDecorationBreak: "clone",
        boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${swatch.destructiveMark} 45%, transparent)`,
        color: swatch.destructiveTintText,
        textDecorationLine: { default: null, [media.forcedColors]: "line-through" },
    },
});

/** Tokens next to each other that changed, or that did not. */
interface Run {
    /** Whether they changed. */
    readonly isWord: boolean;
    /** The tokens. */
    readonly tokens: readonly Token[];
}

/** Render the code of a line in its colors, its changed words marked. */
export function DiffCode(properties: {
    /** The diff. */
    readonly control: DiffControl;
    /** The line. */
    readonly line: DiffLine;
}): JSX.Element {
    // cut the line at its changed words
    const runs = createMemo(() => runsOf(properties.control, properties.line));
    const locale = useLocale();
    const isOpen = (): boolean => {
        // report a last line whose break only one text has
        const { before, after } = properties.line;
        const model = properties.control.model();

        return (
            (before !== undefined && model.isOpen("before", before)) ||
            (after !== undefined && model.isOpen("after", after))
        );
    };
    const look = (): style.Styles =>
        properties.line.kind === "added" ? styles.added : styles.removed;

    return (
        <>
            {runs().map(({ isWord, tokens }) =>
                isWord ? (
                    <span data-word={properties.line.kind} {...style.attrs(look())}>
                        {tokens.map((token) => token.text).join("")}
                    </span>
                ) : (
                    <CodeTokens tokens={tokens} />
                ),
            )}
            <Show when={isOpen()}>
                <span
                    title={locale.render(t`No line break at the end`)}
                    {...style.attrs(styles.open)}
                >
                    <Icon icon={warning} size="0.9em" />
                </span>
            </Show>
        </>
    );
}

/** Read the tokens of a line, cut at the words that changed. */
function runsOf(control: DiffControl, line: DiffLine): readonly Run[] {
    // parse as far as the line
    const side: DiffSide = line.after === undefined ? "before" : "after";
    const model = control.model();
    const source = model[side];
    const range = source.range(line[side] ?? 0);
    const tokens = control.syntax[side].tokens(line[side] ?? 0);

    // read between the changed words and within them by turns
    const runs: Run[] = [];
    let at = range.from;
    const take = (to: number, isWord: boolean): void => {
        if (to > at) {
            runs.push({ isWord, tokens: [...tokens.within({ from: at, to })] });
            at = to;
        }
    };
    for (const word of model.words(line)) {
        const within: Range = {
            from: Math.max(range.from, word.from),
            to: Math.min(range.to, word.to),
        };
        take(within.from, false);
        take(within.to, true);
    }
    take(range.to, false);

    return runs;
}
