import { palette } from "../../palette.stylex";

/** The planet colour of each app and service, chosen by what it does. */
const glyphs: Readonly<Record<string, string>> = {
    // apps
    pages: palette.neptune,
    tasks: palette.sun,
    chat: palette.pulsar,
    source: palette.aurora,
    calendar: palette.mars,
    table: palette.oxygen,
    mail: palette.venus,
    agent: palette.nebula,
    canvas: palette.pulsar,

    // services
    accounts: palette.uranus,
    vault: palette.saturn,
    storage: palette.neptune,
    bucket: palette.venus,
    search: palette.nebula,
    notify: palette.mars,
    sync: palette.pulsar,
    telemetry: palette.oxygen,
    hosts: palette.sun,

    // layers and resources
    user: palette.uranus,
    services: palette.pulsar,
    auth: palette.saturn,
    settings: palette.oxygen,
    ai: palette.nebula,
    audit: palette.mars,
    registry: palette.sun,
    build: palette.saturn,
    template: palette.pulsar,
    computer: palette.neptune,
    cloud: palette.uranus,
    lock: palette.saturn,
    file: palette.neptune,
    presence: palette.nebula,
    card: palette.oxygen,
};

/** The share of its planet colour a glyph keeps on a dark page, the rest lifted toward cream. */
const TILE_SHARE = 90;

/** The share of its planet colour a glyph keeps on a light page, the rest toward ink. */
const RAIL_SHARE = 75;

/** Return the planet colour of a glyph, or refuse a glyph without one. */
export function glyphOf(icon: string): string {
    const glyph = glyphs[icon];
    if (glyph === undefined) {
        throw new RangeError(`no planet colour for glyph ${icon}`);
    }

    return glyph;
}

/** Return the colour a glyph takes on the page, deepened on light and lifted on dark. */
export function paperGlyphOf(icon: string): string {
    const glyph = glyphOf(icon);

    return `light-dark(color-mix(in oklab, ${glyph} ${String(RAIL_SHARE)}%, ${palette.ink}), color-mix(in oklab, ${glyph} ${String(TILE_SHARE)}%, ${palette.cream}))`;
}

/** Return the wash behind a selected glyph on the page. */
export function washOf(icon: string): string {
    return `color-mix(in srgb, ${glyphOf(icon)} 16%, transparent)`;
}
