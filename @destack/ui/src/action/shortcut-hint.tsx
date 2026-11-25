import { Shortcut } from "@destack/locale";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, motion, radius, shadow, space, stroke } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    For,
    type JSX,
    Show,
    STROKE_TIME,
    useActions,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { KbdShortcut } from "../kbd/index.ts";

/** The styles of a hint of shortcuts. */
const styles = style.create({
    hint: {
        position: "fixed",
        insetBlockEnd: space[5],
        insetInlineEnd: space[5],
        zIndex: 3,
        display: "grid",
        gridTemplateColumns: "auto 1fr",
        alignItems: "center",
        columnGap: space[3],
        rowGap: space[2],
        paddingBlock: space[3],
        paddingInline: space[4],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        boxShadow: shadow.overlay,
        opacity: { default: 1, "@starting-style": 0 },
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
});

/** Render what can follow the strokes typed of a longer shortcut, after Vim's which-key. */
export function ShortcutHint(): JSX.Element {
    // forget the strokes once no other follows in time
    const actions = useActions();
    const locale = useLocale();
    const { clock } = useEnvironment();
    const { progress } = actions.sequence;
    createEffect(progress, (typed) => {
        // wait for the next stroke
        if (typed === undefined) {
            return undefined;
        }
        const timer = clock.setTimeout(() => actions.sequence.cancel(), STROKE_TIME);

        return () => clock.clearTimeout(timer);
    });

    // list the strokes that follow, with what each runs
    const next = () => {
        const typed = progress();
        const count = Shortcut.strokes(typed?.typed ?? "").length;

        return (typed?.next ?? []).flatMap(({ key, target }) => {
            const action = actions.find(target);
            const keys = Shortcut.strokes(key).slice(count).join(" ");

            return action === undefined ? [] : [{ keys, title: locale.render(action.title) }];
        });
    };

    return (
        <Show when={next().length > 0}>
            <div role="status" data-slot="shortcut-hint" {...style.attrs(text.sm, styles.hint)}>
                <For each={next()}>
                    {(row) => (
                        <>
                            <KbdShortcut keys={row.keys} />
                            <span>{row.title}</span>
                        </>
                    )}
                </For>
            </div>
        </Show>
    );
}
