import { frame } from "./frame.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { Sky } from "../effect/sky";
import { PartOfSpeech, Syllables } from "../home/entry/entry";
import { lattice, rule } from "./lattice.stylex";
import { headwordBox, headwordDot, headwordLetters } from "../content/generated/headword";
import { Mark } from "./mark";
import { AgentField, TerminalField } from "../install/install";
import { agentPrompt } from "../content/site";
import { release } from "../content/generated/release";
import {
    type Destination,
    isExternal,
    isLive,
    legalDestinations,
    menu,
    type PendingDestination,
    socialDestinations,
} from "./navigation";
import { typeScale, tracking } from "./type.stylex";

/** The id of the hatching the headword's letters are filled with. */
const HATCH_ID = "headword-hatch";

/** The pitch of the headword's hatching in its own units, about 8px across the frame. */
const HATCH_PITCH = 24;

/** The width of each line of the headword's hatching in its own units, about 2.5px. */
const HATCH_LINE = 7;

/** Close every page with a colophon in starry space, ending in the sheet's title block. */
export function Footer(properties: { path: string }) {
    // fill the title block as a drawing's, for the sheet this page is
    const fields = [
        ["Project", "Destack"],
        ["Sheet", properties.path],
        ["Revision", release.version],
        ["Stage", release.stability],
        ["Scale", "1 : 1"],
        ["Units", "CSS px"],
    ] as const;

    return (
        <footer data-component="SiteFooter" {...style.attrs(styles.root)}>
            {/* mark the rails where the footer's top edge meets them */}
            <span aria-hidden="true" {...style.attrs(lattice.crossings, lattice.crossingsTop)} />
            <Sky xstyle={styles.band}>
                {/* spread hairlines down from the footer's top edge, a stack coming apart */}
                <span aria-hidden="true" {...style.attrs(lattice.stave)} />
                <div {...style.attrs(lattice.frame, styles.colophon)}>
                    {/* the verb's dictionary entry, as each section heads with one */}
                    <div {...style.attrs(styles.cell, styles.entry)}>
                        <p {...style.attrs(styles.kicker)}>
                            <Mark />
                            <b {...style.attrs(styles.word)}>
                                <Syllables syllables={["de", "stack"]} />
                            </b>
                            <span>/diːˈstak/</span>
                            <PartOfSpeech>verb</PartOfSpeech>
                        </p>
                        <p {...style.attrs(styles.definition)}>to program the universe</p>
                    </div>

                    {/* the link columns and the install command */}
                    {columns.map(([title, destinations], index) => (
                        <nav
                            aria-label={title}
                            {...style.attrs(
                                styles.cell,
                                styles.column,
                                index === 0 && styles.firstColumn,
                                index === columns.length - 1 && styles.lastColumn,
                            )}
                        >
                            <h2 {...style.attrs(styles.heading)}>{title}</h2>
                            {destinations.map((destination) => (
                                <FooterLink destination={destination} />
                            ))}
                        </nav>
                    ))}
                    <div {...style.attrs(styles.cell, styles.column, styles.install)}>
                        <h2 {...style.attrs(styles.heading)}>Ask your agent, or install</h2>
                        <AgentField prompt={agentPrompt} />
                        <TerminalField />
                    </div>

                    {/* close on the headword itself, its ink touching every edge */}
                    <svg
                        aria-hidden="true"
                        viewBox={`0 0 ${headwordBox.width} ${headwordBox.height}`}
                        {...style.attrs(styles.poster)}
                    >
                        {/* hatch the letters like a cut face, inside their outline */}
                        <defs>
                            <pattern
                                id={HATCH_ID}
                                width={HATCH_PITCH}
                                height={HATCH_PITCH}
                                patternTransform="rotate(-45)"
                                patternUnits="userSpaceOnUse"
                            >
                                <rect
                                    width={HATCH_LINE}
                                    height={HATCH_PITCH}
                                    {...style.attrs(styles.hatch)}
                                />
                            </pattern>
                        </defs>
                        <path
                            d={headwordLetters}
                            fill={`url(#${HATCH_ID})`}
                            {...style.attrs(styles.letters)}
                        />
                        <circle
                            cx={headwordDot.x}
                            cy={headwordDot.y}
                            r={headwordDot.radius}
                            {...style.attrs(styles.dot)}
                        />
                    </svg>

                    {/* sign the sheet off with its title block */}
                    <dl {...style.attrs(styles.block)}>
                        {fields.map(([label, value]) => (
                            <div {...style.attrs(styles.field)}>
                                <dt {...style.attrs(styles.fieldLabel)}>{label}</dt>
                                <dd {...style.attrs(styles.fieldValue)}>{value}</dd>
                            </div>
                        ))}
                    </dl>
                </div>
            </Sky>
        </footer>
    );
}

/** The footer's columns of links: the site's menu, the community and the legal pages. */
const columns: readonly (readonly [
    title: string,
    destinations: readonly (Pick<Destination, "href" | "label"> | PendingDestination)[],
])[] = [
    ["Site", menu],
    ["Community", socialDestinations],
    ["Legal", legalDestinations],
];

/** Draw one footer destination: a link on or off the site, or its name while it is pending. */
function FooterLink(properties: {
    destination: Pick<Destination, "href" | "label"> | PendingDestination;
}) {
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
        position: "relative",
        width: "100%",
        zIndex: 1,
    },
    band: {
        minHeight: frame.bar,
    },
    colophon: {
        color: color.foreground,
        containerType: "inline-size",
        fontFamily: font.text,
    },
    cell: {
        alignContent: "start",
        display: "grid",
        minWidth: 0,
        paddingBlock: { default: "3rem", [media.maxMd]: "2rem" },
        paddingInline: frame.inset,
        rowGap: "0.75rem",
    },
    entry: {
        gridColumn: "1 / -1",
        paddingBlock: { default: "7rem 6rem", [media.maxMd]: "4rem 3rem" },
        rowGap: "1.25rem",
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
        lineHeight: 1.05,
        margin: 0,
    },
    column: {
        // band the column's top, and its left where a column stands beside it
        backgroundImage: `${rule.band}, ${rule.band}`,
        backgroundPosition: "left top, left top",
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.bandHeight}, ${rule.bandHeight} 100%`,
        gridColumn: "span 2",
        justifyItems: "start",
        paddingBlock: { default: "2.5rem 3.5rem", [media.maxMd]: "2rem" },
    },
    firstColumn: {
        backgroundSize: `100% ${rule.bandHeight}, 0 0`,
    },
    lastColumn: {
        backgroundSize: {
            default: `100% ${rule.bandHeight}, ${rule.bandHeight} 100%`,
            [media.maxMd]: `100% ${rule.bandHeight}, 0 0`,
        },
        gridColumn: { default: "span 2", [media.maxMd]: "1 / -1" },
    },
    install: {
        gridColumn: { default: "span 6", [media.maxMd]: "1 / -1" },
        justifyItems: "stretch",
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
        backgroundImage: rule.band,
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.bandHeight}`,
        display: "block",
        gridColumn: "1 / -1",
        width: "100%",
    },
    letters: {
        stroke: color.foreground,
        strokeWidth: 6,
    },
    hatch: {
        fill: color.foreground,
    },
    block: {
        backgroundImage: rule.band,
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.bandHeight}`,
        paddingBlockStart: rule.bandHeight,
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: {
            default: "repeat(6, minmax(0, 1fr))",
            [media.maxMd]: "repeat(2, minmax(0, 1fr))",
        },
        margin: 0,
    },
    field: {
        borderColor: rule.color,
        borderStyle: "solid",
        borderWidth: 0,
        borderInlineStartWidth: { default: stroke.border, ":first-child": 0 },
        display: "grid",
        paddingBlock: "0.875rem",
        paddingInline: frame.inset,
        rowGap: "0.25rem",
    },
    fieldLabel: {
        color: `color-mix(in srgb, ${color.foreground} 55%, transparent)`,
        fontFamily: font.code,
        fontSize: "0.625rem",
        letterSpacing: tracking.label,
        textTransform: "uppercase",
    },
    fieldValue: {
        fontFamily: font.code,
        fontSize: typeScale.label,
        letterSpacing: tracking.label,
        margin: 0,
        overflowWrap: "anywhere",
        textTransform: "uppercase",
    },
    dot: {
        fill: color.primary,
    },
});
