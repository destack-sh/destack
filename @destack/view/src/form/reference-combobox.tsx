import type { RelationalQuery } from "@destack/object/client";
import {
    Combobox,
    ComboboxContent,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
    useCombobox,
} from "@destack/ui/combobox";
import { Spinner } from "@destack/ui/spinner";
import { useQuery } from "../scope/query.ts";
import type { JSX } from "../solid/component.ts";
import { For, Loading } from "../solid/flow.ts";
import { createEffect } from "../solid/reactive.ts";

/** The properties of a reference combobox. */
export interface ReferenceComboboxProperties<Row extends { readonly id: string }> {
    /** Query the objects matching the typed text, such as `space.query.notebook.findMany({ where: … })`. */
    readonly search: (text: string) => RelationalQuery<readonly Row[]>;
    /** Write the text that names an object. */
    readonly label: (row: Row) => string;
    /** The chosen object. */
    readonly value?: Row | undefined;
    /** Handle another object being chosen. */
    readonly onValueChange?: (row: Row) => void;
    /** The message while the text matches no object. */
    readonly empty?: JSX.Element;
    /** The hint inside the empty input. */
    readonly placeholder?: string;
    /** The accessible name of the input. */
    readonly "aria-label"?: string;
}

/** Render a combobox that searches an object type through the client's query and picks one of its objects. */
export function ReferenceCombobox<Row extends { readonly id: string }>(
    properties: ReferenceComboboxProperties<Row>,
): JSX.Element {
    // remember the shown rows to hand the chosen one to its owner
    let shown: readonly Row[] = [];
    const choose = (id: string) => {
        const row = shown.find((entry) => entry.id === id);
        if (row !== undefined) {
            properties.onValueChange?.(row);
        }
    };

    return (
        <Combobox
            shouldFilter={false}
            {...(properties.value === undefined ? {} : { value: properties.value.id })}
            onValueChange={choose}
        >
            <ReferenceInput properties={properties} />
            <ComboboxContent>
                <Loading fallback={<Spinner />}>
                    <ReferenceOptions
                        properties={properties}
                        onRows={(rows) => {
                            shown = rows;
                        }}
                    />
                </Loading>
            </ComboboxContent>
        </Combobox>
    );
}

/** Render the combobox's input, starting from the chosen object's name. */
function ReferenceInput<Row extends { readonly id: string }>(properties: {
    readonly properties: ReferenceComboboxProperties<Row>;
}): JSX.Element {
    // show the chosen object's name whenever the choice changes
    const combobox = useCombobox();
    createEffect(
        () => properties.properties.value,
        (chosen) => {
            if (chosen !== undefined) {
                combobox.list.type(properties.properties.label(chosen));
            }
        },
    );

    return (
        <ComboboxInput
            aria-label={properties.properties["aria-label"]}
            placeholder={properties.properties.placeholder}
            data-slot="reference-combobox-input"
        />
    );
}

/** Render the objects the typed text's query finds as options. */
function ReferenceOptions<Row extends { readonly id: string }>(properties: {
    readonly properties: ReferenceComboboxProperties<Row>;
    readonly onRows: (rows: readonly Row[]) => void;
}): JSX.Element {
    // follow the typed text's query and hand its rows to the combobox
    const combobox = useCombobox();
    const rows = useQuery(() => properties.properties.search(combobox.list.search()));
    createEffect(rows, (list) => {
        properties.onRows(list);
    });

    return (
        <>
            <ComboboxEmpty>{properties.properties.empty}</ComboboxEmpty>
            <For each={rows()}>
                {(row) => (
                    <ComboboxItem value={row.id}>{properties.properties.label(row)}</ComboboxItem>
                )}
            </For>
        </>
    );
}
