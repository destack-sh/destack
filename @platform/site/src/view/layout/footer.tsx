import { frame } from "./frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { Goo } from "../effect/goo";
import { Syllables } from "../home/entry/entry";
import { lattice } from "./lattice.stylex";
import { Mark } from "./mark";
import {
    type Destination,
    isExternal,
    isLive,
    menu,
    type PendingDestination,
    socialDestinations,
} from "./navigation";
import { typeScale, tracking } from "./type.stylex";

/** Close every page with a colophon in starry space: the verb's entry, the links, the headword. */
export function Footer() {
    return (
        <footer data-component="SiteFooter" {...style.attrs(styles.root)}>
            <Goo xstyle={styles.band}>
                <div {...style.attrs(lattice.frame, styles.colophon)}>
                    {/* the verb's dictionary entry, as each section heads with one */}
                    <div {...style.attrs(styles.cell, styles.entry)}>
                        <p {...style.attrs(styles.kicker)}>
                            <Mark />
                            <b {...style.attrs(styles.word)}>
                                <Syllables syllables={["de", "stack"]} />
                            </b>
                            <span>/diːˈstak/</span>
                            <i>verb</i>
                        </p>
                        <p {...style.attrs(styles.definition)}>to reconquer your software</p>
                    </div>

                    {/* the site's menu, and the community off the site */}
                    {columns.map(([title, destinations], index) => (
                        <nav
                            aria-label={title}
                            {...style.attrs(
                                styles.cell,
                                styles.column,
                                index === 0 && styles.firstColumn,
                            )}
                        >
                            <h2 {...style.attrs(styles.heading)}>{title}</h2>
                            {destinations.map((destination) => (
                                <FooterLink destination={destination} />
                            ))}
                        </nav>
                    ))}

                    {/* who makes Destack, its license, and how to reach them */}
                    <div {...style.attrs(styles.fine)}>
                        <span>© 2026 {company.name} · MIT licensed</span>
                        <a href={`mailto:${company.mail}`} {...style.attrs(styles.link)}>
                            Contact
                        </a>
                    </div>

                    {/* close on the headword itself, edge to edge and fading into the sky */}
                    <div aria-hidden="true" {...style.attrs(styles.poster)}>
                        <span {...style.attrs(styles.headword)}>
                            de
                            <span {...style.attrs(styles.dot)} />
                            stack
                        </span>
                    </div>
                </div>
            </Goo>
        </footer>
    );
}

/** The footer's columns of links: the site's menu, and the community off the site. */
const columns: readonly (readonly [
    title: string,
    destinations: readonly (Destination | PendingDestination)[],
])[] = [
    ["Site", menu],
    ["Community", socialDestinations],
];

/** Draw one footer destination: a link on or off the site, or its name while it is pending. */
function FooterLink(properties: { destination: Destination | PendingDestination }) {
    const destination = properties.destination;

    // show a pending destination without a link
    if (!isLive(destination)) {
        return (
            <span title="Soon" {...style.attrs(styles.link, styles.pending)}>
                {destination.label}
            </span>
        );
    }

    return (
        <a
            href={destination.href}
            {...(isExternal(destination)
                ? { rel: "external noopener noreferrer", target: "_blank" }
                : {})}
            {...style.attrs(styles.link)}
        >
            {destination.label}
        </a>
    );
}

/** The company behind Destack, and the address it answers mail at. */
const company = {
    name: "Symbol Industries",
    mail: "florian@symbol.industries",
};

/** The footer styles. */
const styles = style.create({
    root: {
        marginInline: "auto",
        maxWidth: frame.width,
        position: "relative",
        width: "100%",
        zIndex: 1,
    },
    band: {
        minHeight: frame.bar,
    },
    colophon: {
        borderInlineWidth: 0,
        color: color.foreground,
        containerType: "inline-size",
        fontFamily: font.text,
    },
    cell: {
        alignContent: "start",
        borderColor: `color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        borderStyle: "solid",
        borderWidth: 0,
        borderInlineStartWidth: stroke.border,
        display: "grid",
        minWidth: 0,
        paddingBlock: { default: "3rem", [media.maxMd]: "2rem" },
        paddingInline: frame.inset,
        rowGap: "0.75rem",
    },
    entry: {
        borderInlineStartWidth: 0,
        gridColumn: { default: "span 6", [media.maxMd]: "1 / -1" },
    },
    kicker: {
        alignItems: "center",
        color: `color-mix(in srgb, ${color.foreground} 60%, transparent)`,
        columnGap: "0.625rem",
        display: "flex",
        flexWrap: "wrap",
        fontSize: typeScale.body,
        margin: 0,
    },
    word: {
        color: color.primary,
        fontWeight: weight.semibold,
    },
    definition: {
        fontSize: typeScale.display,
        fontWeight: weight.medium,
        letterSpacing: tracking.display,
        lineHeight: 1.15,
        margin: 0,
    },
    column: {
        borderTopWidth: { default: 0, [media.maxMd]: stroke.border },
        gridColumn: { default: "span 3", [media.maxMd]: "span 2" },
        justifyItems: "start",
    },
    firstColumn: {
        borderInlineStartWidth: { default: stroke.border, [media.maxMd]: 0 },
    },
    heading: {
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: weight.regular,
        letterSpacing: tracking.label,
        margin: 0,
        marginBlockEnd: "0.25rem",
        textTransform: "uppercase",
    },
    link: {
        color: {
            default: `color-mix(in srgb, ${color.foreground} 85%, transparent)`,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontSize: typeScale.small,
        textDecorationLine: "none",
    },
    pending: {
        cursor: "default",
    },
    fine: {
        alignItems: "center",
        borderTopColor: `color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        display: "flex",
        fontSize: typeScale.small,
        gridColumn: "1 / -1",
        justifyContent: "space-between",
        paddingBlock: "1.25rem",
        paddingInline: frame.inset,
    },
    poster: {
        gridColumn: "1 / -1",
        overflow: "hidden",
        paddingBlockStart: "1rem",
    },
    headword: {
        color: color.foreground,
        display: "block",
        // fill the frame edge to edge: the word's ink is 3.69em wide at this weight and tracking
        fontSize: "calc(100cqi / 3.69)",
        fontWeight: weight.bold,
        // show the word down to its baseline, which sits 0.775em down a 0.8 line
        height: "0.78em",
        letterSpacing: "-0.045em",
        lineHeight: 0.8,
        // pull the d's left side bearing out past the frame edge
        marginBlockStart: "-0.035em",
        marginInlineStart: "-0.043em",
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    dot: {
        backgroundColor: color.primary,
        borderRadius: "50%",
        display: "inline-block",
        height: "0.13em",
        marginInlineEnd: "0.09em",
        marginInlineStart: "0.07em",
        verticalAlign: "0.19em",
        width: "0.13em",
    },
});
