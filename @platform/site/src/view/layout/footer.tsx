import { frame } from "./frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { Goo } from "../effect/goo";
import { Syllables } from "../home/entry/entry";
import { lattice } from "./lattice.stylex";
import { headwordBox, headwordDot, headwordLetters } from "../content/generated/headword";
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
                        <p {...style.attrs(styles.definition)}>to program the universe</p>
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

                    {/* close on the headword itself, its ink touching every edge */}
                    <svg
                        aria-hidden="true"
                        viewBox={`0 0 ${headwordBox.width} ${headwordBox.height}`}
                        {...style.attrs(styles.poster)}
                    >
                        <path d={headwordLetters} {...style.attrs(styles.letters)} />
                        <circle
                            cx={headwordDot.x}
                            cy={headwordDot.y}
                            r={headwordDot.radius}
                            {...style.attrs(styles.dot)}
                        />
                    </svg>
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
        fontSize: typeScale.closing,
        fontWeight: weight.semibold,
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
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontSize: typeScale.body,
        textDecorationLine: "none",
    },
    pending: {
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        cursor: "default",
    },
    poster: {
        borderTopColor: `color-mix(in srgb, ${color.foreground} 14%, transparent)`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        display: "block",
        gridColumn: "1 / -1",
        width: "100%",
    },
    letters: {
        fill: color.foreground,
    },
    dot: {
        fill: color.primary,
    },
});
