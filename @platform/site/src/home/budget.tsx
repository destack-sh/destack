import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";
import type { Item, Lighting } from "./ledger";
import { Tile } from "./tile";
import { Callout } from "./callout";

/** What you ask for. */
const request = "Track our spending from my bank exports and share the budget with Sam.";

/** Three moments in the life of an app built on request, numbered as its callouts: as a chat artifact, or as an app in your space. */
export const moments: readonly { stacked: Item; destacked: Item }[] = [
    {
        stacked: {
            label: "Build",
            mark: { icon: "file", tint: "#6d7f86" },
            name: "Pasted into the chat",
            chips: ["CSV"],
            note: "your bank export goes along with the prompt",
        },
        destacked: {
            label: "Build",
            mark: { icon: "file", tint: "#b8862b" },
            name: "Read from your files",
            chips: ["allowed once"],
            note: "your agent asks before it opens the export",
        },
    },
    {
        stacked: {
            label: "Share",
            mark: { icon: "auth", tint: "#6d7f86" },
            name: "Anyone with the link",
            chips: ["public"],
            note: "the link shows your spending to whoever has it",
        },
        destacked: {
            label: "Share",
            mark: { icon: "auth", tint: "#a0485f" },
            name: "Sam, by one rule",
            chips: ["Sam reads"],
            note: "Sam sees the budget, never your bank accounts",
        },
    },
    {
        stacked: {
            label: "Change, next month",
            mark: { icon: "sync", tint: "#6d7f86" },
            name: "Regenerate from scratch",
            chips: ["paste again"],
            note: "a new link each time, and the old one still works",
        },
        destacked: {
            label: "Change, next month",
            mark: { icon: "source", tint: "#c64a17" },
            name: "Changed on a branch",
            chips: ["preview", "undo"],
            note: "preview it as Sam, then merge or undo",
        },
    },
];

/** This month's categories: name, spent, budget, and the share spent out of a hundred. */
const categories: readonly (readonly [
    name: string,
    spent: string,
    budget: string,
    share: number,
])[] = [
    ["Rent", "€1,450", "€1,450", 100],
    ["Groceries", "€612", "€700", 87],
    ["Eating out", "€238", "€200", 119],
    ["Transport", "€164", "€250", 66],
    ["Subscriptions", "€96", "€120", 80],
    ["Household", "€280", "€380", 74],
];

/** The latest transactions: date, merchant, category, and amount. */
const transactions: readonly (readonly [
    date: string,
    merchant: string,
    category: string,
    amount: string,
])[] = [
    ["3 Oct", "Lidl", "Groceries", "−€84.20"],
    ["3 Oct", "Uber", "Transport", "−€18.40"],
    ["2 Oct", "Spotify", "Subscriptions", "−€10.99"],
    ["2 Oct", "Notion", "Subscriptions", "−€10.00"],
    ["1 Oct", "Landlord", "Rent", "−€1,450.00"],
    ["30 Sep", "Salary", "Income", "+€4,200.00"],
    ["29 Sep", "Ikea", "Household", "−€126.50"],
    ["28 Sep", "Trattoria Roma", "Eating out", "−€64.00"],
];

/** Draw the month's categories as bars, red where spending ran over. */
function Categories() {
    return (
        <span {...stylex.attrs(styles.categories)}>
            {categories.map(([name, spent, budget, share]) => (
                <span {...stylex.attrs(styles.category)}>
                    <span {...stylex.attrs(styles.categoryHead)}>
                        <span>{name}</span>
                        <span {...stylex.attrs(styles.quiet, styles.figure)}>
                            {spent} / {budget}
                        </span>
                    </span>
                    <span {...stylex.attrs(styles.track)}>
                        <span
                            style={{ width: `${Math.min(share, 100)}%` }}
                            {...stylex.attrs(styles.fill, share > 100 && styles.over)}
                        />
                    </span>
                </span>
            ))}
        </span>
    );
}

/** Draw the latest transactions. */
function Transactions() {
    return (
        <table {...stylex.attrs(styles.table)}>
            <tbody>
                {transactions.map(([date, merchant, category, amount]) => (
                    <tr>
                        <td {...stylex.attrs(styles.cell, styles.date)}>{date}</td>
                        <td {...stylex.attrs(styles.cell, styles.strong)}>{merchant}</td>
                        <td {...stylex.attrs(styles.cell)}>
                            <span {...stylex.attrs(styles.tag)}>{category}</span>
                        </td>
                        <td
                            {...stylex.attrs(
                                styles.cell,
                                styles.figure,
                                amount.startsWith("+") && styles.income,
                            )}
                        >
                            {amount}
                        </td>
                    </tr>
                ))}
            </tbody>
        </table>
    );
}

/** Draw the same budget in both frames: beside the chat it was asked for in, or inside your space, each moment marked by its number. */
export function BudgetApp(properties: { isOpen: boolean; lighting: Lighting }) {
    // mark the place of one moment, dashed while stacked
    const at = (number: number) => ({
        number,
        isOpen: properties.isOpen,
        lighting: properties.lighting,
    });

    return (
        <div {...stylex.attrs(styles.app)}>
            <aside {...stylex.attrs(styles.rail)}>
                {properties.isOpen ? (
                    <>
                        <p {...stylex.attrs(styles.group)}>Your space</p>
                        {["home", "pages", "notes", "tasks"].map((name) => (
                            <p {...stylex.attrs(styles.entry)}>
                                <Tile name={name} />
                                {name.charAt(0).toUpperCase() + name.slice(1)}
                            </p>
                        ))}
                        <p {...stylex.attrs(styles.entry, styles.selected)}>
                            <Tile name="homemade" />
                            Budget
                            <span {...stylex.attrs(styles.branchDot)} />
                        </p>
                        <p {...stylex.attrs(styles.group)}>People</p>
                        <p {...stylex.attrs(styles.entry)}>
                            <span {...stylex.attrs(styles.avatar)}>Y</span>
                            You
                        </p>
                        <p {...stylex.attrs(styles.entry)}>
                            <span {...stylex.attrs(styles.avatar, styles.sam)}>S</span>
                            Sam
                        </p>
                    </>
                ) : (
                    <>
                        <p {...stylex.attrs(styles.group)}>Chat</p>
                        <p {...stylex.attrs(styles.message, styles.mine)}>{request}</p>
                        <p {...stylex.attrs(styles.paste)}>
                            <b {...stylex.attrs(styles.strong)}>bank-october.csv</b>
                            <span {...stylex.attrs(styles.quiet)}>142 lines pasted</span>
                        </p>
                        <p {...stylex.attrs(styles.message)}>
                            Here is your budget. Share the link with Sam.
                        </p>
                    </>
                )}
            </aside>
            <div {...stylex.attrs(styles.frame)}>
                <p {...stylex.attrs(styles.line)}>
                    {properties.isOpen ? (
                        <>
                            <Tile name="agent" />
                            <Callout {...at(1)}>
                                <span>May I read bank-october.csv from your files?</span>
                            </Callout>
                            <span {...stylex.attrs(plate.plate, styles.answer)}>Allowed once</span>
                        </>
                    ) : (
                        <>
                            <Tile name="you" />
                            <Callout {...at(1)}>
                                <span>Your bank export, pasted into the chat</span>
                            </Callout>
                            <span {...stylex.attrs(plate.plate, plate.closed, styles.answer)}>
                                142 lines
                            </span>
                        </>
                    )}
                </p>
                <p {...stylex.attrs(styles.bar)}>
                    <span {...stylex.attrs(styles.strong)}>
                        {properties.isOpen ? "Budget · branch budget" : "budget.html · artifact"}
                    </span>
                    <span {...stylex.attrs(styles.end)}>
                        <Callout {...at(2)}>
                            <span {...stylex.attrs(styles.button)}>
                                {properties.isOpen ? "Shared with Sam" : "Share link"}
                            </span>
                        </Callout>
                        <Callout {...at(3)}>
                            <span {...stylex.attrs(styles.pair)}>
                                {properties.isOpen ? (
                                    <>
                                        <span {...stylex.attrs(styles.button)}>Preview</span>
                                        <span {...stylex.attrs(styles.button, styles.primary)}>
                                            Merge
                                        </span>
                                    </>
                                ) : (
                                    <span {...stylex.attrs(styles.button)}>Regenerate</span>
                                )}
                            </span>
                        </Callout>
                    </span>
                </p>
                <div {...stylex.attrs(styles.body)}>
                    <b {...stylex.attrs(styles.title)}>October</b>
                    <Categories />
                    <Transactions />
                </div>
            </div>
        </div>
    );
}

/** The budget styles. */
const styles = stylex.create({
    avatar: {
        alignItems: "center",
        backgroundColor: "#2f7d8c",
        borderRadius: "50%",
        color: "#ffffff",
        display: "inline-flex",
        fontSize: "0.6rem",
        fontWeight: 700,
        height: "1.125rem",
        justifyContent: "center",
        width: "1.125rem",
    },
    message: {
        lineHeight: 1.45,
        margin: 0,
    },
    mine: {
        backgroundColor: color.card,
        borderRadius: "10px",
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    paste: {
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.7rem",
        gap: "0.125rem",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    rail: {
        alignContent: "start",
        backgroundColor: color.muted,
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        display: { default: "grid", "@media (max-width: 767px)": "none" },
        gap: "0.375rem",
        minHeight: 0,
        overflow: "hidden",
        padding: "0.75rem",
    },
    frame: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
    },
    group: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        margin: 0,
        paddingBlock: "0.25rem",
        textTransform: "uppercase",
    },
    entry: {
        alignItems: "center",
        borderRadius: "5px",
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        marginInline: "-0.375rem",
        paddingBlock: "0.1875rem",
        paddingInline: "0.375rem",
    },
    selected: {
        backgroundColor: "rgb(255 121 46 / 14%)",
        fontWeight: 600,
    },
    branchDot: {
        backgroundColor: tokens.signal,
        borderRadius: "50%",
        height: "0.375rem",
        marginLeft: "auto",
        width: "0.375rem",
    },
    sam: {
        backgroundColor: "#a0485f",
    },
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: {
            default: "11rem minmax(0, 1fr)",
            "@media (max-width: 767px)": "minmax(0, 1fr)",
        },
        height: "100%",
        minHeight: 0,
    },
    line: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "0.625rem",
        lineHeight: 1.4,
        margin: 0,
        paddingBlock: "0.625rem",
        paddingInline: "1.25rem",
    },
    answer: {
        marginLeft: "auto",
    },
    body: {
        alignContent: "start",
        display: "grid",
        gap: "0.75rem",
        maskImage: "linear-gradient(to bottom, #000 calc(100% - 3rem), transparent)",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: "0.875rem",
        paddingInline: { default: "1.5rem", "@media (max-width: 767px)": "1rem" },
    },
    title: {
        fontSize: "1.375rem",
        fontWeight: 700,
        letterSpacing: "-0.02em",
    },
    end: {
        alignItems: "center",
        display: "flex",
        gap: "1.25rem",
        marginLeft: "auto",
    },
    categories: {
        columnGap: "1.5rem",
        display: "grid",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        padding: "0.25rem",
        rowGap: "0.625rem",
    },
    category: {
        display: "grid",
        gap: "0.25rem",
    },
    categoryHead: {
        display: "flex",
        justifyContent: "space-between",
    },
    track: {
        backgroundColor: color.muted,
        borderRadius: "999px",
        display: "block",
        height: "0.375rem",
        overflow: "hidden",
    },
    fill: {
        backgroundColor: "#2f7d8c",
        display: "block",
        height: "100%",
    },
    over: {
        backgroundColor: "#c0392b",
    },
    table: {
        borderCollapse: "collapse",
        width: "100%",
    },
    cell: {
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        paddingBlock: "0.375rem",
        paddingRight: "0.75rem",
        whiteSpace: "nowrap",
    },
    date: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.72rem",
        paddingLeft: "0.25rem",
        width: "4rem",
    },
    tag: {
        backgroundColor: color.muted,
        borderRadius: "999px",
        color: color.mutedForeground,
        fontSize: "0.7rem",
        paddingBlock: "0.0625rem",
        paddingInline: "0.5rem",
    },
    figure: {
        fontVariantNumeric: "tabular-nums",
        textAlign: "right",
    },
    income: {
        color: "#3c8f58",
        fontWeight: 600,
    },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        color: color.mutedForeground,
    },
    button: {
        borderColor: tokens.rule,
        borderRadius: "5px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.foreground,
        fontSize: "0.75rem",
        fontWeight: 600,
        paddingBlock: "0.125rem",
        paddingInline: "0.625rem",
        whiteSpace: "nowrap",
    },
    primary: {
        backgroundColor: tokens.signal,
        borderColor: tokens.signal,
        color: tokens.signalInk,
    },
    bar: {
        alignItems: "center",
        borderBottomColor: tokens.rule,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "flex",
        gap: "0.75rem",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "1.25rem",
    },
    pair: {
        display: "flex",
        gap: "0.5rem",
    },
});
