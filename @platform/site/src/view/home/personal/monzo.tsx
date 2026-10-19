import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { monzo } from "./monzo.stylex";

/** Your latest payments by day: merchant, category, amount, logo colour, and whether it came in. */
const days: readonly (readonly [
    day: string,
    payments: readonly (readonly [
        merchant: string,
        category: string,
        amount: string,
        tint: string,
        isIncome: boolean,
    ])[],
])[] = [
    ["Tuesday 21 October", [["Hetzner", "Bills", "16.40", "#d50c2d", false]]],
    ["Monday 20 October", [["Workshop Coffee", "Eating out", "3.80", "#3d2b1f", false]]],
    [
        "Saturday 18 October",
        [
            ["Trainline", "Transport", "42.10", "#01c3a7", false],
            ["Jonas Weber", "Splits", "12.00", "#6b5ca5", true],
        ],
    ],
];

/** The payment open in the detail pane. */
const CHOSEN = "Workshop Coffee";

/** Draw your Monzo web feed: your balance and payments, one opened beside them. */
export function Monzo() {
    return (
        <div data-component="MonzoPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.head)}>
                <span
                    style={{ "mask-image": "url(/diagram/monzo.svg)" }}
                    {...style.attrs(styles.logo)}
                />
                <span {...style.attrs(styles.account)}>Personal account</span>
                <span {...style.attrs(styles.me)}>Florian · Log out</span>
            </header>

            <div {...style.attrs(styles.panes)}>
                {/* feed */}
                <section {...style.attrs(styles.feed)}>
                    <div {...style.attrs(styles.balance)}>
                        <span {...style.attrs(styles.figure)}>
                            <b {...style.attrs(styles.amount)}>£1,284.50</b>
                            <span {...style.attrs(styles.muted)}>Balance</span>
                        </span>
                        <span {...style.attrs(styles.figure)}>
                            <b {...style.attrs(styles.amountSmall)}>£0.00</b>
                            <span {...style.attrs(styles.muted)}>Spent today</span>
                        </span>
                    </div>
                    {days.map(([day, payments]) => (
                        <>
                            <span {...style.attrs(styles.day)}>{day}</span>
                            {payments.map(([merchant, category, amount, tint, isIncome]) => (
                                <span
                                    {...style.attrs(
                                        styles.payment,
                                        merchant === CHOSEN && styles.chosen,
                                    )}
                                >
                                    <span
                                        style={{ "background-color": tint }}
                                        {...style.attrs(styles.logoCircle)}
                                    >
                                        {merchant.slice(0, 1)}
                                    </span>
                                    <span {...style.attrs(styles.what)}>
                                        <b>{merchant}</b>
                                        <span {...style.attrs(styles.muted)}>{category}</span>
                                    </span>
                                    <b {...style.attrs(styles.money, isIncome && styles.income)}>
                                        {isIncome ? `+${amount}` : amount}
                                    </b>
                                </span>
                            ))}
                        </>
                    ))}
                </section>

                {/* detail */}
                <section {...style.attrs(styles.detail)}>
                    <svg
                        viewBox="0 0 300 110"
                        preserveAspectRatio="xMidYMid slice"
                        {...style.attrs(styles.map)}
                    >
                        <rect width="300" height="110" {...style.attrs(styles.land)} />
                        <path
                            d="M0 86 C 80 70, 160 100, 300 80 V110 H0Z"
                            {...style.attrs(styles.water)}
                        />
                        <path
                            d="M0 40 H300 M120 0 V110 M210 0 L250 110 M0 64 L300 52"
                            {...style.attrs(styles.road)}
                        />
                        <circle cx="150" cy="50" r="7" {...style.attrs(styles.pin)} />
                    </svg>
                    <span
                        style={{ "background-color": "#3d2b1f" }}
                        {...style.attrs(styles.bigLogo)}
                    >
                        W
                    </span>
                    <b {...style.attrs(styles.detailName)}>Workshop Coffee</b>
                    <span {...style.attrs(styles.muted)}>Clerkenwell, London</span>
                    <b {...style.attrs(styles.detailAmount)}>£3.80</b>
                    <span {...style.attrs(styles.muted)}>Monday 20 October, 08:41</span>
                    <span {...style.attrs(styles.rows)}>
                        {[
                            ["Category", "Eating out"],
                            ["Notes", "Add a note or #tag"],
                            ["Receipt", "Add receipt"],
                        ].map(([label, value]) => (
                            <span {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.muted)}>{label}</span>
                                <span>{value}</span>
                            </span>
                        ))}
                    </span>
                </section>
            </div>
        </div>
    );
}

/** The Monzo page styles. */
const styles = style.create({
    page: {
        backgroundColor: monzo.surface,
        color: monzo.ink,
        display: "grid",
        fontFamily: monzo.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    head: {
        alignItems: "center",
        backgroundColor: monzo.navy,
        color: monzo.onDark,
        display: "flex",
        gap: "14px",
        height: "50px",
        paddingInline: { default: "20px", [site.narrow]: "12px" },
    },
    logo: {
        backgroundColor: monzo.coral,
        height: "24px",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "24px",
    },
    account: {
        fontWeight: 600,
    },
    me: {
        display: { default: "inline", [site.narrow]: "none" },
        fontSize: "12px",
        marginInlineStart: "auto",
        opacity: 0.75,
    },
    panes: {
        display: "grid",
        gap: "12px",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
        overflow: "hidden",
        padding: "12px",
    },
    feed: {
        alignContent: "start",
        backgroundColor: monzo.pane,
        borderRadius: "10px",
        display: "grid",
        overflow: "hidden",
    },
    balance: {
        borderBottomColor: monzo.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "24px",
        padding: "16px",
    },
    figure: {
        display: "grid",
        fontSize: "11px",
        gap: "2px",
    },
    amount: {
        fontSize: "24px",
        fontWeight: 700,
        letterSpacing: "-0.02em",
    },
    amountSmall: {
        fontSize: "24px",
        fontWeight: 400,
        letterSpacing: "-0.02em",
    },
    day: {
        backgroundColor: monzo.surface,
        color: monzo.muted,
        fontSize: "11px",
        fontWeight: 600,
        paddingBlock: "5px",
        paddingInline: "16px",
    },
    payment: {
        alignItems: "center",
        borderBottomColor: monzo.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "12px",
        paddingBlock: "9px",
        paddingInline: "16px",
    },
    chosen: {
        backgroundColor: monzo.chosen,
    },
    logoCircle: {
        alignItems: "center",
        borderRadius: "50%",
        color: monzo.onDark,
        display: "inline-flex",
        flexShrink: 0,
        fontWeight: 700,
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    what: {
        display: "grid",
        gap: "1px",
    },
    money: {
        fontSize: "14px",
        marginInlineStart: "auto",
    },
    income: {
        color: monzo.income,
    },
    detail: {
        alignContent: "start",
        backgroundColor: monzo.pane,
        borderRadius: "10px",
        display: { default: "grid", [site.narrow]: "none" },
        justifyItems: "center",
        overflow: "hidden",
        paddingBlockEnd: "14px",
        rowGap: "3px",
        textAlign: "center",
    },
    map: {
        height: "100px",
        width: "100%",
    },
    land: {
        fill: monzo.land,
    },
    water: {
        fill: monzo.water,
    },
    road: {
        fill: "none",
        stroke: monzo.road,
        strokeWidth: 5,
    },
    pin: {
        fill: monzo.coral,
        stroke: monzo.road,
        strokeWidth: 3,
    },
    bigLogo: {
        alignItems: "center",
        borderColor: monzo.pane,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "3px",
        color: monzo.onDark,
        display: "inline-flex",
        fontSize: "20px",
        fontWeight: 700,
        height: "52px",
        justifyContent: "center",
        marginBlockStart: "-26px",
        width: "52px",
    },
    detailName: {
        fontSize: "16px",
        paddingBlockStart: "4px",
    },
    detailAmount: {
        fontSize: "28px",
        fontWeight: 700,
        paddingBlockStart: "6px",
    },
    rows: {
        display: "grid",
        paddingBlockStart: "10px",
        paddingInline: "16px",
        textAlign: "start",
        width: "100%",
    },
    row: {
        borderTopColor: monzo.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "8px",
    },
    muted: {
        color: monzo.muted,
    },
});
