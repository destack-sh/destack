import { plural, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show, useLocale } from "@destack/view";
import { Toggle, type ToggleProperties } from "../toggle/index.ts";
import { Toolbar, type ToolbarProperties } from "../toolbar/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";

/** The people a reaction lists before it counts the rest. */
const LISTED = 10;

/** The styles of reactions. */
const styles = style.create({
    reactions: {
        flexWrap: "wrap",
        gap: space[1],
        padding: 0,
        borderWidth: 0,
    },
    reaction: {
        gap: space[1],
        minWidth: 0,
        height: size[1],
        paddingInline: space[2],
        borderWidth: stroke.border,
        borderColor: {
            default: "transparent",
            ":is([aria-pressed=true])": {
                default: swatch.accentMark,
                [media.forcedColors]: "Highlight",
            },
        },
        borderRadius: radius.full,
        backgroundColor: {
            default: color.muted,
            ":is([aria-pressed=true])": {
                default: swatch.accentTint,
                [media.forcedColors]: "Highlight",
            },
        },
        color: {
            default: null,
            ":is([aria-pressed=true])": {
                default: swatch.accentTintText,
                [media.forcedColors]: "HighlightText",
            },
        },
        forcedColorAdjust: { default: null, ":is([aria-pressed=true])": "none" },
        fontWeight: "inherit",
    },
    count: {
        fontVariantNumeric: "tabular-nums",
    },
});

/** Render the reactions to something as one Tab stop. */
export function ReactionGroup(properties: ToolbarProperties): JSX.Element {
    const locale = useLocale();

    return (
        <Toolbar
            data-slot="reaction-group"
            aria-label={locale.render(t`Reactions`)}
            {...properties}
            xstyle={[styles.reactions, properties.xstyle]}
        />
    );
}

/** The properties of a reaction, its toggle's included. */
export type ReactionProperties = Omit<ToggleProperties, "children" | "variant" | "size"> & {
    /** The emoji. */
    readonly emoji: string;
    /** How many people reacted with it. */
    readonly count: number;
    /** Who reacted with it, shown on hover, none by default. */
    readonly people?: readonly string[];
    /** What it shows, its emoji and count by default. */
    readonly children?: JSX.Element;
};

/** Render one reaction, which a press adds one's own to or takes back. */
export function Reaction(properties: ReactionProperties): JSX.Element {
    // say the emoji, how many and who
    const locale = useLocale();
    const rest = omit(properties, "emoji", "count", "people", "children", "xstyle");
    const people = (): string => {
        const all = properties.people ?? [];
        const more = Math.max(properties.count, all.length) - LISTED;

        return locale.list([
            ...all.slice(0, LISTED),
            ...(more > 0 && all.length > 0
                ? [locale.render(t`${plural(more, { one: "# other", other: "# others" })}`)]
                : []),
        ]);
    };
    const toggle = (attributes: object): JSX.Element => (
        <Toggle
            {...attributes}
            data-slot="reaction"
            aria-label={locale.render(
                t`${properties.emoji}, ${plural(properties.count, { one: "# person", other: "# people" })}`,
            )}
            {...rest}
            xstyle={[text.xs, state.touch, styles.reaction, properties.xstyle]}
        >
            {properties.children ?? (
                <>
                    <span>{properties.emoji}</span>
                    <span {...style.attrs(styles.count)}>{locale.number(properties.count)}</span>
                </>
            )}
        </Toggle>
    );

    return (
        <Show when={(properties.people ?? []).length > 0} fallback={toggle({})}>
            <Tooltip>
                <TooltipTrigger render={(attributes) => toggle(attributes)} />
                <TooltipContent>{people()}</TooltipContent>
            </Tooltip>
        </Show>
    );
}
