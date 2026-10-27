import { color, font, motion } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuLabel,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";
import { For, type JSX } from "@destack/view";

import { Favicon } from "./glyph";
import { typeScale, tracking } from "../../layout/type.stylex";

/** The mark of an option: a shape from the icon set in its tint, or the vendor's own picture. */
export type OptionMark = { icon: string; tint: string } | { logo: string };

/** One option a picker turns through: a product's name and mark. */
export type Option = { name: string; mark: OptionMark };

/** A heading in a picker's menu and the names of the options under it. */
export type OptionSection = readonly [label: string, names: readonly string[]];

/**
 * Draw a small picker sized to sit in a figure's label row, like the dial.
 *
 * The shown option turns on its own while a hairline fills, and its caret opens every option.
 */
export function Picker(properties: {
    label: string;
    options: readonly Option[];
    sections: readonly OptionSection[] | undefined;
    shown: string;
    progress: number | undefined;
    onChoose: (name: string) => void;
}) {
    return (
        <span data-component="Picker" {...style.attrs(styles.picker)}>
            <span aria-hidden="true">{properties.label}</span>
            <DropdownMenu>
                <DropdownMenuTrigger
                    variant="ghost"
                    size="sm"
                    aria-label={`${properties.label}: ${properties.shown}`}
                    xstyle={styles.trigger}
                >
                    <span {...style.attrs(styles.names)}>
                        <For each={properties.options}>
                            {(option) => (
                                <Name
                                    option={option}
                                    isShown={option.name === properties.shown}
                                    isMarked
                                    isFitted={false}
                                />
                            )}
                        </For>
                    </span>
                    <svg aria-hidden="true" viewBox="0 0 24 24" {...style.attrs(styles.caret)}>
                        <path d="m6 9 6 6 6-6" />
                    </svg>
                    <Hairline progress={properties.progress} />
                </DropdownMenuTrigger>
                <OptionMenu
                    options={properties.options}
                    sections={properties.sections}
                    shown={properties.shown}
                    onChoose={properties.onChoose}
                />
            </DropdownMenu>
        </span>
    );
}

/** Wrap a row's mark and name in a picker: pressing either opens the menu of every option. */
export function InlinePicker(properties: {
    label: string;
    options: readonly Option[];
    shown: string;
    onChoose: (name: string) => void;
    children: JSX.Element;
}) {
    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                variant="ghost"
                aria-label={`${properties.label}: ${properties.shown}`}
                xstyle={styles.inline}
            >
                {properties.children}
            </DropdownMenuTrigger>
            <OptionMenu
                options={properties.options}
                sections={undefined}
                shown={properties.shown}
                onChoose={properties.onChoose}
            />
        </DropdownMenu>
    );
}

/** Draw the shown option's name, rolling to the next as it changes, and the caret that opens it. */
export function PickerName(properties: { options: readonly Option[]; shown: string }) {
    return (
        <span {...style.attrs(styles.pickerName)}>
            <span {...style.attrs(styles.names)}>
                <For each={properties.options}>
                    {(option) => (
                        <Name
                            option={option}
                            isShown={option.name === properties.shown}
                            isMarked={false}
                            isFitted
                        />
                    )}
                </For>
            </span>
            <svg
                data-caret
                aria-hidden="true"
                viewBox="0 0 24 24"
                {...style.attrs(styles.caret, styles.inlineCaret)}
            >
                <path d="m6 9 6 6 6-6" />
            </svg>
        </span>
    );
}

/** Draw the menu of every option, the shown one checked. */
function OptionMenu(properties: {
    options: readonly Option[];
    sections: readonly OptionSection[] | undefined;
    shown: string;
    onChoose: (name: string) => void;
}) {
    return (
        <DropdownMenuContent align="start" xstyle={styles.menu}>
            <DropdownMenuRadioGroup value={properties.shown} onValueChange={properties.onChoose}>
                {/* list every option, under its section's heading when the picker has sections */}
                {properties.sections === undefined ? (
                    <For each={properties.options}>
                        {(option) => <OptionItem option={option} />}
                    </For>
                ) : (
                    <For each={properties.sections}>
                        {([label, names]) => (
                            <DropdownMenuGroup>
                                <DropdownMenuLabel xstyle={styles.heading}>
                                    {label}
                                </DropdownMenuLabel>
                                <For
                                    each={properties.options.filter((option) =>
                                        names.includes(option.name),
                                    )}
                                >
                                    {(option) => <OptionItem option={option} />}
                                </For>
                            </DropdownMenuGroup>
                        )}
                    </For>
                )}
            </DropdownMenuRadioGroup>
        </DropdownMenuContent>
    );
}

/** Draw one option in a picker's menu: its mark and name. */
function OptionItem(properties: { option: Option }) {
    return (
        <DropdownMenuRadioItem value={properties.option.name} xstyle={styles.item}>
            <Mark mark={properties.option.mark} size={12} />
            {properties.option.name}
        </DropdownMenuRadioItem>
    );
}

/** Draw an option's mark at a size in pixels. */
export function Mark(properties: { mark: OptionMark; size: number }) {
    return (
        <>
            {"icon" in properties.mark ? (
                <Favicon
                    icon={properties.mark.icon}
                    tint={properties.mark.tint}
                    size={properties.size}
                />
            ) : (
                <img
                    alt=""
                    src={`/logos/${properties.mark.logo}`}
                    style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
                    {...style.attrs(styles.logo)}
                />
            )}
        </>
    );
}

/**
 * Draw one option's mark and name, rolled into place while shown and out of it otherwise.
 *
 * A fitted picker sizes to the shown name alone, the others rolling over it out of the layout.
 */
function Name(properties: {
    option: Option;
    isShown: boolean;
    isMarked: boolean;
    isFitted: boolean;
}) {
    return (
        <span
            aria-hidden={properties.isShown ? undefined : "true"}
            {...style.attrs(
                styles.name,
                properties.isShown && styles.nameShown,
                properties.isFitted && !properties.isShown && styles.nameAway,
            )}
        >
            {properties.isMarked ? <Mark mark={properties.option.mark} size={11} /> : undefined}
            {properties.option.name}
        </span>
    );
}

/** Draw the hairline under the shown option, filled as far as its turn has run. */
function Hairline(properties: { progress: number | undefined }) {
    return (
        <span
            aria-hidden="true"
            style={{ transform: `scaleX(${properties.progress ?? 0})` }}
            {...style.attrs(styles.hairline, properties.progress === undefined && styles.still)}
        />
    );
}

/** The picker styles. */
const styles = style.create({
    picker: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: typeScale.label,
        gap: "0.5rem",
        letterSpacing: tracking.label,
        marginBlock: "-0.25rem",
        textTransform: "uppercase",
    },
    trigger: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: color.foreground,
        fontFamily: "inherit",
        fontSize: typeScale.label,
        gap: "0.375rem",
        height: "auto",
        lineHeight: "1rem",
        overflow: "hidden",
        paddingBlock: "1px",
        paddingInline: "0.375rem",
        position: "relative",
        textTransform: "inherit",
    },
    names: {
        display: "inline-grid",
        justifyItems: "start",
        position: "relative",
    },
    name: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.375rem",
        gridArea: "1 / 1",
        opacity: 0,
        transform: "translateY(70%)",
        transitionDuration: motion.durationMedium,
        transitionProperty: "opacity, transform",
        transitionTimingFunction: motion.easingEmphasised,
        whiteSpace: "nowrap",
    },
    nameShown: {
        opacity: 1,
        transform: "translateY(0)",
    },
    nameAway: {
        insetInlineStart: 0,
        position: "absolute",
    },
    caret: {
        fill: "none",
        flexShrink: 0,
        height: "10px",
        opacity: 0.6,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2.5,
        width: "10px",
    },
    hairline: {
        backgroundColor: color.primary,
        bottom: 0,
        height: "1px",
        insetInline: 0,
        opacity: 0.7,
        position: "absolute",
        transformOrigin: "left",
        transitionDuration: { default: "250ms", [media.motionReduce]: "0ms" },
        transitionProperty: "transform",
        transitionTimingFunction: "linear",
    },
    still: {
        opacity: 0,
    },
    inline: {
        alignItems: "center",
        alignSelf: "stretch",
        backgroundColor: "transparent",
        borderRadius: 0,
        color: "inherit",
        display: "flex",
        flexGrow: 1,
        font: "inherit",
        gap: "inherit",
        height: "auto",
        justifyContent: "flex-start",
        letterSpacing: "inherit",
        minWidth: 0,
        paddingBlock: 0,
        paddingInline: 0,
        textAlign: "start",
    },
    pickerName: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.25rem",
        minWidth: 0,
    },
    inlineCaret: {
        opacity: { default: 0.35, ":hover": { default: null, [media.hover]: 0.8 } },
    },
    logo: {
        borderRadius: "3px",
        flexShrink: 0,
    },
    menu: {
        fontSize: typeScale.small,
        minWidth: "9rem",
    },
    item: {
        gap: "0.5rem",
    },
    heading: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        textTransform: "uppercase",
    },
});
