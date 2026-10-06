import {
    constructTable,
    type RowData,
    type Table,
    type TableFeatures,
    type TableOptions,
    type TableState,
} from "@tanstack/table-core";
import { storeReactivityBindings } from "@tanstack/table-core/store-reactivity-bindings";
import { type Accessor, createEffect, createSignal, onCleanup, untrack } from "@destack/view";

/** A headless table whose state its components track. */
export type DataTableInstance<Features extends TableFeatures, Row extends RowData> = Table<
    Features,
    Row
> & {
    /** The table's state, read to track every change of its state and options. */
    readonly tracked: Accessor<TableState<Features>>;
};

/** Create a headless table over reactive options, such as a component's properties, whose state and options its components track. */
export function createTable<Features extends TableFeatures, Row extends RowData>(
    options: TableOptions<Features, Row>,
): DataTableInstance<Features, Row> {
    // run the table on synchronous atoms so its own reads see its writes at once
    const table = constructTable<Features, Row>({
        ...untrack(() => ({ ...options })),
        features: {
            ...options.features,
            coreReactivityFeature: storeReactivityBindings(),
        },
    });

    // mirror the state into a signal on every change of the state or the options
    const [tracked, setTracked] = createSignal<{ readonly state: TableState<Features> }>(
        { state: table.store.state },
        { ownedWrite: true },
    );
    const publish = (): void => {
        setTracked({ state: table.store.state });
    };
    const subscriptions = [table.store.subscribe(publish), table.optionsStore?.subscribe(publish)];
    onCleanup(() => {
        for (const subscription of subscriptions) {
            subscription?.unsubscribe();
        }
    });

    // hand the table each change of the reactive options
    createEffect(
        () => ({ ...options, state: { ...options.state } }),
        (next) =>
            table.setOptions((previous) => ({ ...previous, ...next, features: previous.features })),
    );

    return Object.assign(table, { tracked: () => tracked().state });
}
