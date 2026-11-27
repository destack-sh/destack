import type { Choice } from "@destack/object";
import * as style from "@destack/style";
import { swatches } from "@destack/theme/swatch";
import { radius, space } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show } from "@destack/view";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of a mention. */
const styles = style.create({
    mention: (fill: string, ink: string) => ({
        display: "inline-flex",
        alignItems: "baseline",
        gap: `calc(${space[1]} / 2)`,
        paddingInline: space[1],
        borderRadius: radius[2],
        backgroundColor: fill,
        color: ink,
        fontWeight: 500,
        whiteSpace: "nowrap",
        verticalAlign: "baseline",
        forcedColorAdjust: "none",
    }),
    icon: {
        alignSelf: "center",
    },
});

/** The properties of a mention, the native element's attributes included. */
export type MentionProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class" | "children"> &
    ElementPartProperties & {
        /** The person or object mentioned, as it looks now. */
        readonly choice: Choice;
    };

/** Render a mention in text: the person's or object's icon and name, tinted by its color. */
export function Mention(properties: MentionProperties): JSX.Element {
    // tint by the choice's color, else by the accent
    const rest = omit(properties, "choice");
    const swatch = () => swatches[properties.choice.color ?? "accent"];

    return renderPart(
        "span",
        "mention",
        rest,
        () => styles.mention(swatch().tint, swatch().tintText),
        {
            get children() {
                return (
                    <>
                        {/* Icon, else an at sign */}
                        <Show when={properties.choice.icon !== undefined} fallback="@">
                            <span {...style.attrs(styles.icon)}>
                                <ChoiceIcon choice={properties.choice} />
                            </span>
                        </Show>

                        {/* Name */}
                        {properties.choice.title}
                    </>
                );
            },
        },
    );
}
