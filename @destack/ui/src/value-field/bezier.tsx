import { t } from "@destack/locale";
import { CubicBezier, Easing, EASINGS } from "@destack/style";
import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { For, type JSX, Show, useLocale } from "@destack/view";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** The key of a curve CSS names with no keyword, which the menu lists by its numbers. */
const CUSTOM = "custom";

/** The styles of an easing's input. */
const styles = style.create({
    trigger: {
        width: "100%",
    },
    item: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    curve: {
        flexShrink: 0,
        width: space[4],
        height: space[4],
        overflow: "visible",
        fill: "none",
        stroke: color.foreground,
        strokeWidth: stroke.border,
        strokeLinecap: "round",
    },
});

/** Render the input of an easing: the curves CSS names, each drawn, and a custom one. */
export function CubicBezierInput(properties: NamedValueInputProperties): JSX.Element {
    // read the curve, an ease out where none is given, by its keyword where CSS names it
    const locale = useLocale();
    const curve = (): CubicBezier =>
        readValue(properties.value, CubicBezier) ?? EASINGS["ease-out"].curve;
    const key = (): string => CubicBezier.easing(curve()) ?? CUSTOM;
    const choose = (next: string): void => {
        const easing = Easing.safeParse(next).data;
        if (easing !== undefined) {
            properties.onValueChange([...EASINGS[easing].curve]);
        }
    };

    return (
        <Select
            xstyle={styles.trigger}
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            value={key()}
            onValueChange={(next) => next !== undefined && choose(next)}
        >
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <For each={Easing.options}>
                    {(easing) => (
                        <SelectItem value={easing}>
                            <span {...style.attrs(styles.item)}>
                                <CurveIcon curve={EASINGS[easing].curve} />
                                {locale.render(EASINGS[easing].title)}
                            </span>
                        </SelectItem>
                    )}
                </For>
                <Show when={key() === CUSTOM}>
                    <SelectItem value={CUSTOM}>
                        <span {...style.attrs(styles.item)}>
                            <CurveIcon curve={curve()} />
                            {locale.render(t.context("option")`Custom`)}
                        </span>
                    </SelectItem>
                </Show>
            </SelectContent>
        </Select>
    );
}

/** Draw an easing curve in a square, time across and progress up. */
function CurveIcon(properties: {
    /** The curve. */
    readonly curve: CubicBezier;
}): JSX.Element {
    // place the control points in a unit square, progress up from the bottom
    const path = (): string => {
        const [x1, y1, x2, y2] = properties.curve;

        return `M0 1 C${String(x1)} ${String(1 - y1)} ${String(x2)} ${String(1 - y2)} 1 0`;
    };

    return (
        <svg viewBox="0 0 1 1" aria-hidden="true" {...style.attrs(styles.curve)}>
            <path d={path()} vector-effect="non-scaling-stroke" />
        </svg>
    );
}
