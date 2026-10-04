import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";

import { commandEvents } from "../command/command";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Entry, type Form } from "./entry";

/** The media query for screens narrower than the desktop frame, where the parts pair up. */
const narrow = "@media (min-width: 768px) and (max-width: 1099px)";
/** The media query for phone-width screens, where the parts stack. */
const mobile = "@media (max-width: 767px)";

/** The noun. */
const noun: Form = {
    syllables: ["De", "stack"],
    pronunciation: "/ˈdiːstak/",
    partOfSpeech: "noun",
    senses: [
        {
            definition: "a standardised software stack, absurdly integrated",
            highlight: ["standardised", "absurdly integrated"],
            sentence:
                "Accounts, access, database, workflows, storage, hosting and six more parts, built on one data plane and one software model.",
        },
        {
            definition: "the open source engine that runs it",
            highlight: ["open source"],
            sentence: "Every part below is built on it, and all of its code is yours to read.",
        },
    ],
};

/** A product a part relates to: its name and the address of its square mark. */
type Product = {
    /** The product's name. */
    name: string;
    /** The address of its mark. */
    mark: string;
};

/** One part of the stack, set as a small dictionary entry: headword, definition, and the products it relates to. */
type Part = {
    /** The headword. */
    name: string;
    /** The index of the stack figure's layer the part belongs to. */
    layer: number;
    /** How the part relates to the products: it stands in for them, runs on them, or sends to them. */
    relation: "like" | "on" | "into";
    /** The products, as alternatives or, when combined, as one. */
    products: readonly Product[];
    /** Whether the products combine into one, as GitHub and npm do. */
    isCombined?: boolean;
    /** The headword's colour. */
    tint: string;
    /** What the part does, in one line. */
    definition: string;
};

/** The label each relation sets before its products. */
const relations: Record<Part["relation"], string> = {
    like: "syn.",
    on: "runs on",
    into: "sends to",
};

/** Return a product by name, with its mark from the site's logos. */
function product(name: string, mark: string): Product {
    return { name, mark: `/logos/${mark}.png` };
}

/** The twelve parts, in alphabetical order. */
const parts: readonly Part[] = [
    {
        name: "access",
        layer: 0,
        relation: "like",
        products: [product("SpiceDB", "spicedb"), product("Cedar", "cedar")],
        tint: "#a0485f",
        definition: "roles on every object, and every decision explained",
    },
    {
        name: "accounts",
        layer: 0,
        relation: "like",
        products: [product("Clerk", "clerk"), product("WorkOS", "workos")],
        tint: "#2f7d8c",
        definition: "passkeys, organisations, devices and tokens for agents",
    },
    {
        name: "database",
        layer: 3,
        relation: "like",
        products: [product("Convex", "convex"), product("ElectricSQL", "electricsql")],
        tint: "#2f7d8c",
        definition: "live queries, offline writes, SQLite and Postgres",
    },
    {
        name: "forge",
        layer: 4,
        relation: "like",
        products: [product("GitHub", "github"), product("npm", "npm")],
        isCombined: true,
        tint: "#c64a17",
        definition: "git hosting, builds, releases and a registry of whole apps",
    },
    {
        name: "history",
        layer: 4,
        relation: "like",
        products: [product("Time Machine", "timemachine"), product("Dolt", "dolt")],
        tint: "#5b7f2e",
        definition: "every call recorded, with undo, restore and branches",
    },
    {
        name: "hosting",
        layer: 5,
        relation: "like",
        products: [product("Vercel", "vercel"), product("Fly.io", "fly")],
        tint: "#3d6fb0",
        definition: "your laptop, a server or our cloud, joined by tunnels",
    },
    {
        name: "multiplayer",
        layer: 1,
        relation: "like",
        products: [product("Liveblocks", "liveblocks"), product("PartyKit", "partykit")],
        tint: "#6b5ca5",
        definition: "presence, comments and collaborative text on every object",
    },
    {
        name: "notifications",
        layer: 1,
        relation: "like",
        products: [product("Knock", "knock"), product("Novu", "novu")],
        tint: "#b0567f",
        definition: "inbox, email and push, with preferences per person",
    },
    {
        name: "secrets",
        layer: 2,
        relation: "like",
        products: [product("Doppler", "doppler"), product("Infisical", "infisical")],
        tint: "#9a6b3f",
        definition: "a key per vault, and credentials injected at the edge",
    },
    {
        name: "storage",
        layer: 3,
        relation: "on",
        products: [product("S3", "s3"), product("R2", "r2")],
        tint: "#b8862b",
        definition: "buckets beside your data, and files addressed by digest",
    },
    {
        name: "telemetry",
        layer: 5,
        relation: "into",
        products: [product("Sentry", "sentry"), product("Axiom", "axiom")],
        tint: "#6d7f86",
        definition: "logs, traces and metrics, each span tied to its code",
    },
    {
        name: "workflows",
        layer: 2,
        relation: "like",
        products: [product("Inngest", "inngest"), product("Temporal", "temporal")],
        tint: "#4f8a5b",
        definition: "triggers on time and on change, and runs with retries",
    },
];

/** Show a layer in the stack figure. */
function showLayer(layer: number) {
    document.dispatchEvent(new CustomEvent(commandEvents.showLayer, { detail: layer }));
}

/** List the twelve parts as an alphabetical glossary under the noun. */
export function Parts() {
    return (
        <section data-universe {...stylex.attrs(lattice.frame, lattice.ruleBottom)}>
            <Entry form={noun} />
            {parts.map((part) => (
                <button
                    type="button"
                    title="Show its layer in the stack"
                    onClick={() => showLayer(part.layer)}
                    {...stylex.attrs(styles.part)}
                >
                    <b style={{ color: part.tint }} {...stylex.attrs(styles.headword)}>
                        {part.name}
                    </b>
                    <span {...stylex.attrs(styles.definition)}>{part.definition}</span>
                    <span {...stylex.attrs(styles.products)}>
                        <i {...stylex.attrs(styles.muted)}>{relations[part.relation]}</i>
                        {part.products.map((item, at) => (
                            <span {...stylex.attrs(styles.product)}>
                                {at > 0 && part.isCombined === true && "+ "}
                                <img alt="" src={item.mark} {...stylex.attrs(styles.mark)} />
                                {item.name}
                            </span>
                        ))}
                    </span>
                </button>
            ))}
        </section>
    );
}

/** The parts styles. */
const styles = stylex.create({
    part: {
        alignContent: "start",
        backgroundColor: { default: "transparent", ":hover": "rgb(255 121 46 / 6%)" },
        borderBottomWidth: 0,
        borderColor: tokens.rule,
        borderLeftWidth: 0,
        borderRightWidth: {
            default: tokens.hairline,
            ":nth-of-type(3n)": 0,
            [narrow]: { default: tokens.hairline, ":nth-of-type(2n)": 0 },
            [mobile]: 0,
        },
        borderStyle: "solid",
        borderTopWidth: tokens.hairline,
        color: "inherit",
        cursor: "pointer",
        display: "grid",
        fontFamily: "inherit",
        gridColumn: "span 4",
        minWidth: 0,
        paddingBlock: "1.5rem",
        paddingInline: tokens.inset,
        rowGap: "0.5rem",
        textAlign: "left",
        transitionDuration: "150ms",
        transitionProperty: "background-color",
        [narrow]: { gridColumn: "span 6" },
        [mobile]: { gridColumn: "1 / -1" },
    },
    headword: {
        fontSize: "1.1875rem",
        fontWeight: 700,
    },
    muted: {
        color: color.mutedForeground,
        fontSize: "0.875rem",
    },
    definition: {
        fontSize: "1rem",
        lineHeight: 1.45,
        minHeight: "2.9em",
        textWrap: "pretty",
    },
    products: {
        alignItems: "center",
        display: "flex",
        flexWrap: "wrap",
        fontSize: "0.875rem",
        gap: "0.25rem 0.625rem",
    },
    product: {
        alignItems: "center",
        display: "inline-flex",
        gap: "0.3rem",
        whiteSpace: "nowrap",
    },
    mark: {
        borderRadius: "3px",
        height: "0.9375rem",
        width: "0.9375rem",
    },
});
