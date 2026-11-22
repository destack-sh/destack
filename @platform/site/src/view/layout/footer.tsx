import { measure } from "./measure.stylex";
import { media } from "@destack/style/media.stylex";
import { color, font, stroke, weight, swatch } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";

import { Sky } from "../effect/sky";
import { Mark, planet } from "./mark";
import { tagline } from "../content/site";
import { Entry, type Form } from "../home/entry/entry";
import { lattice, rule } from "./lattice.stylex";
import { headwordBox, headwordDot, headwordLetters } from "../content/generated/headword";
import InstallIsland from "../islands/install";
import {
    type Destination,
    isExternal,
    legalDestinations,
    menu,
    socialDestinations,
} from "./navigation";
import { typeScale, tracking } from "./type.stylex";

/** The headword's letters as a mask, so their hatching has the bands' pitch at every width. */
const LETTERS_MASK = `url("data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${headwordBox.width} ${headwordBox.height}"><path d="${headwordLetters}"/></svg>`,
)}")`;

/** The mark's box across, scaled so its planet covers the headword's dot. */
const DOT_MARK = (headwordDot.radius / planet.radius) * 2 * planet.centre.x;

/** The mark standing in for the headword's dot, placed and sized in shares of the headword. */
const DOT_PLANET = {
    left: `${(headwordDot.x / headwordBox.width) * 100}%`,
    top: `${(headwordDot.y / headwordBox.height) * 100}%`,
    width: `${(DOT_MARK / headwordBox.width) * 100}%`,
};

/** The verb the footer closes on, in the sense of the whole universe. */
const closing: Form = {
    syllables: ["de", "stack"],
    pronunciation: "dee-stack",
    partOfSpeech: "verb",
    sense: { definition: "to program the universe", highlight: [], sentence: tagline },
};

/** Close every page with a colophon in the sky, ending on the headword flush with the bottom. */
export function Footer() {
    return (
        <footer data-component="SiteFooter" {...style.attrs(styles.root)}>
            {/* mark the rails where the footer's top edge meets them */}
            <span aria-hidden="true" {...style.attrs(lattice.crossings, lattice.crossingsTop)} />
            <Sky view="plate" xstyle={styles.band}>
                {/* spread hairlines down from the footer's top edge, a stack coming apart */}
                <span aria-hidden="true" {...style.attrs(lattice.stave)} />
                <div {...style.attrs(lattice.frame, styles.colophon)}>
                    {/* the verb's dictionary entry, as each section heads with one */}
                    <Entry form={closing} isAnchor={false} />

                    {/* the link columns and the ways to install */}
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
                        <h2 {...style.attrs(styles.heading)}>Install</h2>
                        <InstallIsland />
                    </div>

                    {/* close on the headword itself, its ink touching every edge, the planet as its dot */}
                    <div {...style.attrs(styles.poster)}>
                        {/* hatch the letters like a cut face, at the pitch of every band */}
                        <span
                            aria-hidden="true"
                            style={{ "mask-image": LETTERS_MASK }}
                            {...style.attrs(styles.hatch)}
                        />

                        {/* outline the letters in a hairline of one width at every size */}
                        <svg
                            aria-hidden="true"
                            viewBox={`0 0 ${headwordBox.width} ${headwordBox.height}`}
                            {...style.attrs(styles.letterbox)}
                        >
                            <path
                                d={headwordLetters}
                                vector-effect="non-scaling-stroke"
                                {...style.attrs(styles.letters)}
                            />
                        </svg>
                        <span style={DOT_PLANET} {...style.attrs(styles.planet)}>
                            <Mark xstyle={styles.mark} />
                        </span>
                    </div>
                </div>
            </Sky>
        </footer>
    );
}

/** The footer's columns of links: the site's menu, the community and the legal pages. */
const columns: readonly (readonly [
    title: string,
    destinations: readonly Pick<Destination, "href" | "label">[],
])[] = [
    ["Site", menu],
    ["Community", socialDestinations],
    ["Legal", legalDestinations],
];

/** Draw one footer destination: a link on or off the site. */
function FooterLink(properties: { destination: Pick<Destination, "href" | "label"> }) {
    const destination = properties.destination;

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
        minHeight: measure.bar,
    },
    colophon: {
        color: color.foreground,
        containerType: "inline-size",
        fontFamily: font.text,
        paddingBlockStart: `calc(${rule.staveHeight} + 1rem)`,
    },
    cell: {
        alignContent: "start",
        display: "grid",
        minWidth: 0,
        paddingBlock: { default: "3rem", [media.maxMd]: "2rem" },
        paddingInline: measure.inset,
        rowGap: "0.75rem",
    },
    column: {
        // band the column's top, and rule its left where a column stands beside it
        backgroundImage: rule.band,
        backgroundPosition: "left top",
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.bandHeight}`,
        borderLeftColor: rule.color,
        borderLeftStyle: "solid",
        borderLeftWidth: stroke.border,
        gridColumn: "span 2",
        justifyItems: "start",
        paddingBlock: { default: "2.5rem 3.5rem", [media.maxMd]: "2rem" },
    },
    firstColumn: {
        borderLeftWidth: 0,
    },
    lastColumn: {
        borderLeftWidth: { default: stroke.border, [media.maxMd]: 0 },
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
            ":hover": { default: null, [media.hover]: swatch.accentSolid },
        },
        fontSize: typeScale.body,
        textDecorationLine: "none",
    },
    poster: {
        backgroundImage: rule.band,
        backgroundRepeat: "no-repeat",
        backgroundSize: `100% ${rule.bandHeight}`,
        gridColumn: "1 / -1",
        position: "relative",
    },
    letterbox: {
        display: "block",
        position: "relative",
        width: "100%",
    },
    planet: {
        position: "absolute",
        transform: "translate(-50%, -50%)",
    },
    mark: {
        height: "auto",
        width: "100%",
    },
    hatch: {
        backgroundImage: rule.band,
        inset: 0,
        maskRepeat: "no-repeat",
        maskSize: "100% 100%",
        position: "absolute",
    },
    letters: {
        fill: "none",
        stroke: color.foreground,
        strokeWidth: stroke.border,
    },
});
