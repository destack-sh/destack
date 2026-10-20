import { present } from "@destack/schema";
import type { JSX } from "@destack/view";

import { isEmptyOf, type Item, type Lighting, type Line, marksOf } from "./item";
import { type Cell, type Column, Ledger, type LedgerRow } from "./ledger";
import type { Option } from "./picker";
import { type Scale, usageAt, usageCostOf } from "./price";
import type { Stagger } from "./stagger";
import { palette } from "../../palette.stylex";
import { rates } from "../../content/generated/pricing";

/** The bytes in a gigabyte. */
const GB = 1_000_000_000;

/** The format of every figure: to the hundredth, thousands separated, so decimals line up. */
const decimals = new Intl.NumberFormat("en-US", {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
});

/** The ledger's columns, each headed once with its unit. */
const columns: readonly Column[] = [
    {
        caption: "Database",
        unit: "GB",
        note: "Database storage, hot",
        isQuiet: true,
    },
    { caption: "Files", unit: "GB", note: "File storage, cold", isQuiet: true },
    { caption: "Compute", unit: "GB·h", note: "Memory time", isQuiet: true },
    { caption: "Cost", unit: "$/mo", note: "US dollars a month", isQuiet: false },
];

/** Draw a figure's bill as a ledger of blocks: database, files, compute and cost. */
export function Bill(properties: {
    isWindow: boolean;
    lines: readonly Line[];
    scale: Scale;
    scales: readonly Scale[];
    isOpenAt: Stagger;
    lighting?: Lighting;
    onToggle?: (row: number) => void;
    badgeOf?: (row: number) => JSX.Element;
    optionsOf?: (row: number) => readonly Option[];
    onChoose?: (row: number, name: string) => void;
}) {
    // press a block: destack its line alone, or choose it, or let it go when it is the one lit
    const press = (index: number) => {
        const row = index + 1;
        if (properties.onToggle !== undefined) {
            properties.onToggle(row);
        } else if (properties.lighting !== undefined) {
            properties.lighting.onChoose(properties.lighting.chosen === row ? undefined : row);
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
                plan: isOwned(index) ? "Billed per usage" : planOf(item),
                isEmpty: isEmptyOf(line.entry.stacked.mark) && !isOwned(index),
                badge: isOwned(index) ? properties.badgeOf?.(index + 1) : undefined,
                options: isOwned(index) ? [] : (properties.optionsOf?.(index + 1) ?? []),
                cells: cellsOf(line, properties.scale, isOwned(index)),
            };
        });

    // size each column to its widest figure at every dial setting in both states, so nothing shifts
    const widths = () =>
        columns.map((_, column) =>
            Math.max(
                ...properties.scales.flatMap((scale) =>
                    properties.lines.flatMap((line) =>
                        [false, true].map((isInSpace) =>
                            lengthOf(present(cellsOf(line, scale, isInSpace)[column], "cell")),
                        ),
                    ),
                ),
            ),
        );

    return (
        <Ledger
            isWindow={properties.isWindow}
            columns={columns}
            widths={widths()}
            rows={rows()}
            isWhole={isWhole()}
            lit={lightOf(properties.lighting)}
            isPressable={properties.lighting !== undefined || properties.onToggle !== undefined}
            onPoint={(index) => properties.lighting?.onLight(rowOf(index))}
            onPress={press}
            onChoose={(index, name) => properties.onChoose?.(index + 1, name)}
        />
    );
}

/** Return a meter's amount to the hundredth, a floor for what rounds to nothing, or a dash. */
function meterOf(amount: number, isFloor: boolean): Cell {
    // mark nothing with a dash and a trace with a floor
    if (amount === 0) {
        return { value: "—", isQuiet: true, isFloor: false, caption: undefined };
    } else if (amount < 0.01) {
        return { value: "<0.01", isQuiet: true, isFloor: false, caption: undefined };
    }

    return { value: decimalOf(amount), isQuiet: true, isFloor, caption: undefined };
}

/** Return a line's figures for a team and audience, as rented or as run in your space. */
function cellsOf(line: Line, scale: Scale, isOwned: boolean): readonly Cell[] {
    // price the line's work at your space's rates, or at the vendor's list price
    const usage = usageAt(line.workload, scale);
    const rent = costOf(line.entry.stacked, scale);
    const cost = isOwned ? usageCostOf(usage, rates) : rent;

    // leave the cost blank where nothing does the job or another subscription bills it
    const isBlank = isEmptyOf(line.entry.stacked.mark) || cost === undefined;

    // set the change against the rent above an owned cost, where there is a rent to compare
    const caption =
        isOwned && !isBlank && rent !== undefined && rent > 0 ? changeOf(cost, rent) : undefined;

    return [
        meterOf(usage.database / GB, false),
        meterOf(usage.files / GB, false),
        meterOf(usage.memory, line.workload.isAtLeast),
        { value: isBlank ? "—" : decimalOf(cost), isQuiet: false, isFloor: false, caption },
    ];
}

/** Return a cost's change against the rent as a whole percent, a saving rounded toward none. */
function changeOf(cost: number, rent: number): string {
    // floor the saving, so a saving short of everything never reads as a hundred percent
    const saving = Math.floor((1 - cost / rent) * 100);

    return saving >= 0 ? `\u2212${saving}%` : `+${-saving}%`;
}

/** Return the characters a figure takes, its floor mark included. */
function lengthOf(cell: Cell): number {
    return cell.value.length + (cell.isFloor ? 1 : 0);
}

/** Return the ledger row a lighting's lit line sits on, counting from zero. */
function lightOf(lighting: Lighting | undefined): number | undefined {
    return lighting?.lit === undefined ? undefined : lighting.lit - 1;
}

/** Return the line a ledger row stands for, counting from one, or none. */
function rowOf(index: number | undefined): number | undefined {
    return index === undefined ? undefined : index + 1;
}

/** Return an amount to the hundredth with thousands separated, so decimal points line up. */
function decimalOf(amount: number): string {
    return decimals.format(amount);
}

/** Return the plan a rented item is on, set as the start of a sentence. */
function planOf(item: Item): string {
    // read the plan's basis and list price, or say the item comes with the rest
    const price = item.price;
    const basis =
        price.kind === "included"
            ? "included"
            : price.kind === "fee"
              ? `${price.basis}, ${price.listed}`
              : price.basis;

    return `${basis.charAt(0).toUpperCase()}${basis.slice(1)}`;
}

/** Return what a rented item costs a team a month, or nothing where it is bundled or included. */
function costOf(item: Item, scale: Scale): number | undefined {
    return item.price.kind === "fee" ? item.price.monthlyAt(scale) : undefined;
}
