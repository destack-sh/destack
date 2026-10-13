import type { JSX } from "@destack/view";

import { isEmptyOf, type Item, type Lighting, type Line, marksOf } from "./item";
import { type Cell, Ledger, type LedgerRow } from "./ledger";
import { usageCostOf } from "./price";
import type { Stagger } from "./stagger";
import { palette } from "../../palette.stylex";
import { rates } from "../../content/generated/rates";

/** The bytes in a gigabyte, the unit a bill counts storage in. */
const GB = 1_000_000_000;

/** Draw a figure's bill as a ledger of equal blocks, each with its storage, compute and cost. */
export function Bill(properties: {
    lines: readonly Line[];
    isOpenAt: Stagger;
    lighting?: Lighting;
    onToggle?: (row: number) => void;
    badgeOf?: (row: number) => JSX.Element;
}) {
    // press a block: destack its line alone, or choose it, or let it go when it is the one lit
    const press = (index: number) => {
        const row = index + 1;
        if (properties.onToggle !== undefined) {
            properties.onToggle(row);
        } else if (properties.lighting !== undefined) {
            properties.lighting.onChoose(properties.lighting.lit === row ? undefined : row);
        }
    };

    // read which lines run in your space, and join the blocks into one once all of them do
    const isOwned = (index: number) => properties.isOpenAt(index + 1);
    const isWhole = () => properties.lines.every((_, index) => isOwned(index));

    // set each line as a ledger row in the state it is in
    const rows = (): readonly LedgerRow[] =>
        properties.lines.map((line, index) => {
            const item = isOwned(index) ? line.entry.destacked : line.entry.stacked;

            return {
                label: line.entry.destacked.label,
                name: item.name,
                marks: marksOf(item.mark),
                tint: isOwned(index) ? palette.signal : line.tint,
                note: item.note,
                isEmpty: isEmptyOf(line.entry.stacked.mark) && !isOwned(index),
                badge: isOwned(index) ? properties.badgeOf?.(index + 1) : undefined,
                cells: [
                    figureOf(
                        "Storage",
                        "GB a month",
                        amountOf(line.workload.storage / GB),
                        true,
                        true,
                    ),
                    {
                        ...figureOf(
                            "Compute",
                            "GB·h of memory time",
                            amountOf(line.workload.compute),
                            true,
                            true,
                        ),
                        isFloor: line.workload.isAtLeast,
                    },
                    figureOf(
                        "Cost",
                        "US dollars a month",
                        isOwned(index)
                            ? dollarsOf(usageCostOf(line.workload, rates))
                            : costOf(line.entry.stacked),
                        false,
                        false,
                    ),
                ],
            };
        });

    return (
        <Ledger
            rows={rows()}
            isWhole={isWhole()}
            lit={lightOf(properties.lighting)}
            isPressable={properties.lighting !== undefined || properties.onToggle !== undefined}
            onPoint={(index) => properties.lighting?.onLight(rowOf(index))}
            onPress={press}
        />
    );
}

/** Return a ledger figure. */
function figureOf(
    caption: string,
    unit: string,
    value: string,
    isQuiet: boolean,
    isWide: boolean,
): Cell {
    return { caption, unit, value, isQuiet, isWide, isFloor: false };
}

/** Return the ledger row a lighting's lit line sits on, counting from zero. */
function lightOf(lighting: Lighting | undefined): number | undefined {
    return lighting?.lit === undefined ? undefined : lighting.lit - 1;
}

/** Return the line a ledger row stands for, counting from one, or none. */
function rowOf(index: number | undefined): number | undefined {
    return index === undefined ? undefined : index + 1;
}

/** Return an amount to the hundredth, a floor for what rounds to nothing, or a dash. */
function amountOf(amount: number): string {
    if (amount === 0) {
        return "—";
    } else if (amount < 0.01) {
        return "<0.01";
    }

    return amount.toFixed(2);
}

/** Return dollars to the cent, so every amount takes the same width. */
function dollarsOf(amount: number): string {
    return `$${amount.toFixed(2)}`;
}

/** Return what a rented item costs a month, or a dash for what is billed by use or apart. */
function costOf(item: Item): string {
    return item.price.kind === "fee" && item.price.monthly > 0
        ? dollarsOf(item.price.monthly)
        : "—";
}
