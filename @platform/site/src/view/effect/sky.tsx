import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";

import { night } from "../theme.ts";
import { palette } from "../palette.stylex";
import { frame } from "../layout/frame.stylex";
import { surface } from "../layout/lattice.stylex";
import { Mark } from "../layout/mark";
import { type Body, ORBITS, writeSky } from "./ephemeris";

/** The site's home, Zurich, in degrees north and east. */
const ZURICH = { latitude: 47.37, longitude: 8.54 } as const;

/** The milliseconds between the sky's updates, a minute: under a pixel of drift. */
const SKY_INTERVAL = 60_000;

/** The star plate: the Yale Bright Star Catalogue to magnitude 5, with the ecliptic. */
const PLATE = `url("/sky.svg")`;

/** The sidereal time the head script writes, in hours, from 0 to 24. */
const HOURS = "var(--sky-hours, 0)";

/** The Moon's lit share the head script writes, from -1 to 1, positive when lit on the east. */
const MOON_LIGHT = "var(--sky-moon-light, 1)";

/** The plate's left edge in its layer, which starts a viewport left of the band's centre. */
const PLATE_X = `calc(100vw - (24 - ${HOURS}) / 24 * ${frame.width})`;

/** The height of the whole plate in the strip, 180 degrees at the scale of 360 across the frame. */
const STRIP_HEIGHT = `calc(${frame.width} / 2)`;

/** The plate's top in the strip, which centres the strip on the zenith of Zurich. */
const STRIP_Y = `calc(${frame.bar} / 2 - ${(90 - ZURICH.latitude) / 180} * ${STRIP_HEIGHT})`;

/** The planets the plate draws as discs. */
const DISCS = ["mercury", "venus", "mars", "jupiter"] as const satisfies readonly Body[];

/** How much of the sky a band shows: the zenith of Zurich at true scale, or the whole plate. */
export type SkyView = "strip" | "plate";

/** Fill a band with the sky over Zurich now, its meridian on the frame's centre. */
export function Sky(properties: { view: SkyView; children?: JSX.Element; xstyle?: style.Styles }) {
    return (
        <div {...style.attributes([styles.sky, clearCells, properties.xstyle], night)}>
            {/* turn the plate to the sidereal time, and draw where the bodies stand on the whole plate */}
            <span aria-hidden="true" {...style.attrs(styles.layer)}>
                <span {...style.attrs(styles.plate, views[properties.view])} />
                {properties.view === "plate" ? (
                    <>
                        {/* draw the Sun, the Moon and the planets, Saturn as the mark */}
                        <Place body="sun">
                            <span {...style.attrs(styles.sun)} />
                        </Place>
                        <Place body="moon">
                            <span {...style.attrs(styles.moon)} />
                        </Place>
                        {DISCS.map((body) => (
                            <Place body={body}>
                                <span {...style.attrs(styles.disc, tints[body])} />
                            </Place>
                        ))}
                        <Place body="saturn">
                            <Mark xstyle={styles.saturn} />
                        </Place>
                    </>
                ) : undefined}
            </span>
            <div {...style.attrs(styles.content)}>{properties.children}</div>
        </div>
    );
}

/** Place a body's symbol where the head script says it stands. */
function Place(properties: { body: Body; children: JSX.Element }) {
    return (
        <span
            style={{
                left: `calc(50% + var(--sky-${properties.body}-x, 0) * ${frame.width})`,
                top: `calc(var(--sky-${properties.body}-y, 0.5) * 100%)`,
            }}
            {...style.attrs(styles.place)}
        >
            {properties.children}
        </span>
    );
}

/** The lattice cells under the sky, left clear so the sky shows through them. */
const clearCells = style.createTheme(surface, { cell: "transparent" });

/** The sky styles. */
const styles = style.create({
    sky: {
        backgroundColor: palette.space,
        color: color.foreground,
        minWidth: 0,
        position: "relative",
    },
    layer: {
        inset: 0,
        overflow: "hidden",
        pointerEvents: "none",
        position: "absolute",
    },
    plate: {
        backgroundImage: PLATE,
        backgroundPositionX: PLATE_X,
        backgroundRepeat: "repeat-x",
        bottom: 0,
        left: "calc(50% - 100vw)",
        position: "absolute",
        top: 0,
        width: "200vw",
    },
    place: {
        display: "flex",
        position: "absolute",
        transform: "translate(-50%, -50%)",
    },
    disc: {
        backgroundColor: palette.cream,
        borderRadius: "50%",
        height: "6px",
        width: "6px",
    },
    moon: {
        borderColor: `color-mix(in srgb, ${palette.cream} 45%, transparent)`,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: `inset calc(${MOON_LIGHT} * 9px) 0 0 0 ${palette.cream}`,
        boxSizing: "border-box",
        height: "9px",
        width: "9px",
    },
    sun: {
        borderColor: palette.cream,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1.5px",
        backgroundImage: `radial-gradient(circle, ${palette.cream} 0 1.5px, transparent 2px)`,
        boxSizing: "border-box",
        height: "12px",
        width: "12px",
    },
    saturn: {
        height: "20px",
        width: "20px",
    },
    content: {
        height: "100%",
        position: "relative",
    },
});

/** Each planet's colour as it shows to the eye: grey, white, rust and warm. */
const tints = style.create({
    mercury: { backgroundColor: `color-mix(in srgb, ${palette.cream} 70%, ${palette.space})` },
    venus: { backgroundColor: palette.cream },
    mars: { backgroundColor: `color-mix(in srgb, ${palette.cream} 40%, ${color.primary})` },
    jupiter: { backgroundColor: `color-mix(in srgb, ${palette.cream} 80%, ${color.primary})` },
});

/** The plate's size in each view: 24 hours across the frame, at true scale or the band's height. */
const views = style.create({
    strip: {
        backgroundPositionY: STRIP_Y,
        backgroundSize: `${frame.width} ${STRIP_HEIGHT}`,
    },
    plate: {
        backgroundSize: `${frame.width} 100%`,
    },
});

/** The script the page's head runs, so the sky stands right before the first paint. */
export const skyScript = `(${writeSky.toString()})(${JSON.stringify(ORBITS)}, ${ZURICH.longitude}, ${SKY_INTERVAL}, { now: () => Date.now(), setInterval: (callback, interval) => setInterval(callback, interval) }, document.documentElement)`;
