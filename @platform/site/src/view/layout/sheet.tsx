import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { lattice } from "./lattice.stylex";

/** The character code before A, so section one is lettered A. */
const LETTER_BASE = 64;

/** The zone numbers of a sheet's twelve columns. */
const ZONES = Array.from({ length: 12 }, (_, index) => String(index + 1));

/** Return a section's letter, A for the first, as a drawing letters its section cuts. */
function letterOf(section: number): string {
    return String.fromCodePoint(LETTER_BASE + section);
}

/** Space two sections apart with a hatched cut, lettered for the section below it. */
export function Interval(properties: { section?: number; children?: JSX.Element }) {
    return (
        <div aria-hidden="true" {...style.attrs(lattice.interval)}>
            {properties.section !== undefined && <Cut letter={letterOf(properties.section)} />}
            {properties.children}
        </div>
    );
}

/** Mark a section cut at both rails: a hatched block on the cut line, and its letter. */
function Cut(properties: { letter: string }) {
    return (
        <>
            <span {...style.attrs(lattice.cut, lattice.cutStart)}>
                {properties.letter}
                <span {...style.attrs(lattice.cutStroke)} />
            </span>
            <span {...style.attrs(lattice.cut, lattice.cutEnd)}>
                {properties.letter}
                <span {...style.attrs(lattice.cutStroke)} />
            </span>
        </>
    );
}

/** Number the twelve columns along a sheet's top or bottom edge, over a ruler. */
export function Zones(properties: { isBottom?: boolean }) {
    return (
        <div
            aria-hidden="true"
            {...style.attrs(
                lattice.frame,
                lattice.zones,
                properties.isBottom === true && lattice.zonesBottom,
            )}
        >
            {ZONES.map((zone) => (
                <span {...style.attrs(lattice.zone)}>{zone}</span>
            ))}
        </div>
    );
}
