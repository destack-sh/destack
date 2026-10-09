import { Filter, type JsonCondition } from "@destack/db";
import { Icon } from "@destack/icon";
import { Button } from "@destack/ui/button";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { InputGroup, InputGroupAddon, InputGroupInput } from "@destack/ui/input-group";
import { itemVariants } from "@destack/ui/item";
import { createMemo, createSignal, type JSX, Show } from "@destack/view";

/** The most rows a page lists, and the rows each older page adds. */
export const PAGE_ROWS = 100;

/** The layouts of the telemetry pages: a header over a list beside the open row. */
export const styles = style.create({
    page: {
        display: "grid",
        gridTemplateRows: "auto 1fr",
        gap: space[3],
        minHeight: "100vh",
        padding: space[4],
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    split: {
        display: "grid",
        gridTemplateColumns: "minmax(20rem, 28rem) 1fr",
        gap: space[3],
        minHeight: 0,
    },
    list: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        margin: 0,
        padding: 0,
        overflowY: "auto",
        listStyle: "none",
    },
    row: {
        width: "100%",
        textAlign: "start",
    },
    detail: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        overflowY: "auto",
    },
    facts: {
        display: "grid",
        gridTemplateColumns: "auto 1fr",
        gap: space[2],
        margin: 0,
    },
    actions: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[2],
    },
    muted: {
        color: color.mutedForeground,
    },
});

/** Read a filter as a person types it: over an object type's fields, or over an event kind's keys and their dotted entries when given none. */
export function useFilter(fields?: ReadonlySet<string>) {
    // parse the text against the fields
    const [typed, type] = createSignal("");
    const condition = createMemo((): JsonCondition | undefined => {
        // parse the text, marking it invalid while it does not parse
        try {
            return typed().trim() === ""
                ? {}
                : Filter.parse(
                      typed(),
                      fields === undefined ? { isRelational: false } : { fields },
                  );
        } catch {
            return undefined;
        }
    });

    return {
        /** The text typed. */
        text: typed,
        /** The condition, an empty one for no text, absent while the text does not parse. */
        condition,
        /** The text a service reads as a filter, empty for none, absent while it does not parse. */
        where: (): string | undefined => (condition() === undefined ? undefined : typed().trim()),
        /** Replace the text. */
        type: (value: string) => void type(value),
    };
}

/** Render the search field of a filter, marked invalid while its text does not parse. */
export function FilterField(properties: {
    /** The filter the field types. */
    filter: ReturnType<typeof useFilter>;
    /** An example filter. */
    placeholder: string;
}): JSX.Element {
    return (
        <InputGroup>
            <InputGroupAddon>
                <Icon name="magnifying-glass" />
            </InputGroupAddon>
            <InputGroupInput
                aria-label="Filter"
                placeholder={properties.placeholder}
                aria-invalid={properties.filter.condition() === undefined ? "true" : undefined}
                value={properties.filter.text()}
                onInput={(event) => properties.filter.type(event.currentTarget.value)}
            />
        </InputGroup>
    );
}

/** Render a list row a person opens, marked while it is the open one. */
export function OpenButton(properties: {
    /** The slot naming the row's kind. */
    slot: string;
    /** Whether the row is the open one. */
    isOpen: boolean;
    /** Open the row. */
    open: () => void;
    /** The row's content. */
    children: JSX.Element;
}): JSX.Element {
    return (
        <button
            type="button"
            data-slot={properties.slot}
            aria-current={properties.isOpen ? "true" : undefined}
            onClick={() => properties.open()}
            {...style.attrs(
                itemVariants({ variant: properties.isOpen ? "muted" : "default", size: "sm" }),
                styles.row,
            )}
        >
            {properties.children}
        </button>
    );
}

/** Render the button reading the next older page while older rows remain. */
export function MoreButton(properties: {
    /** Read the next older page, absent once no older row remains. */
    more: (() => void) | undefined;
}): JSX.Element {
    return (
        <Show when={properties.more}>
            {(more) => (
                <Button variant="outline" size="sm" onClick={() => more()()}>
                    Load older
                </Button>
            )}
        </Show>
    );
}

/** Write a time in Unix milliseconds as people read it in their locale. */
export function timeOf(time: number): string {
    return new Date(time).toLocaleString();
}
