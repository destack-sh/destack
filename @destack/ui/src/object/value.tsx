import checkIcon from "@destack/icon/phosphor/check";
import type { Choice, ObjectValue } from "@destack/object";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import type { Localization } from "@destack/locale";
import { type JSX, useLocale } from "@destack/view";
import { ChoiceIcon } from "../action/choice-icon.tsx";
import { Icon } from "../icon/index.ts";
import { ObjectChip } from "./object.tsx";

/** The text an absent value shows, a dash muted beside present ones. */
const ABSENT = "–";

/** The styles of a value: a choice's icon beside its title, and absence muted. */
const styles = style.create({
    choice: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    absent: {
        color: color.mutedForeground,
    },
    number: {
        fontVariantNumeric: "tabular-nums",
    },
});

/** The properties of a field's value. */
export interface ValueProperties {
    /** The value, as its field's type reads it. */
    readonly value: ObjectValue;
    /** The object a reference or a subject points to, as a choice, absent to show its key. */
    readonly related?: Choice | undefined;
}

/** Render one field's value by its kind: a choice by its icon and title, an object as its chip. */
export function Value(properties: ValueProperties): JSX.Element {
    const locale = useLocale();

    return <>{draw(properties.value, properties.related, locale)}</>;
}

/** Draw a value by its kind, in a reader's locale. */
function draw(value: ObjectValue, related: Choice | undefined, locale: Localization): JSX.Element {
    // write text as it is, a number in the reader's format and a time as their date
    if (value.kind === "text") {
        return <span {...style.attrs(styles.title)}>{value.value}</span>;
    } else if (value.kind === "number") {
        return (
            <span {...style.attrs(styles.number)}>{locale.number(value.value, value.format)}</span>
        );
    } else if (value.kind === "time") {
        return <span>{locale.date(value.value, { month: "short", day: "numeric" })}</span>;
    }
    // check a true boolean
    else if (value.kind === "boolean" && value.value) {
        return <Icon icon={checkIcon} />;
    }
    // show a choice or a state by its icon and title
    else if (value.kind === "choice" || value.kind === "state") {
        return (
            <span {...style.attrs(styles.choice)}>
                <ChoiceIcon choice={value} />
                <span {...style.attrs(styles.title)}>{value.title}</span>
            </span>
        );
    }
    // show another object or a principal as its chip, else by its key
    else if (value.kind === "reference" || value.kind === "subject") {
        const icon = related?.icon;
        const glyph = icon === undefined || "file" in icon ? undefined : icon;

        return (
            <ObjectChip
                title={related?.title ?? value.value}
                {...(glyph === undefined ? {} : { glyph })}
                {...(related?.color === undefined ? {} : { color: related.color })}
            />
        );
    }
    // show structure as its JSON, muted
    else if (value.kind === "json") {
        return (
            <span {...style.attrs(styles.absent, styles.title)}>{JSON.stringify(value.value)}</span>
        );
    }

    return <span {...style.attrs(styles.absent)}>{ABSENT}</span>;
}
