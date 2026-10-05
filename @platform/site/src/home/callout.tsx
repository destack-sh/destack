import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import { type Lighting, numberOf } from "./ledger";

/** Mark one part of an app figure with a small number, outlined while pointed at: dashed while rented, solid once owned. */
export function Callout(properties: {
    number: number;
    isOpen: boolean;
    lighting: Lighting;
    isBlock?: boolean;
    children: JSX.Element;
}) {
    const isLit = () => properties.lighting.lit === properties.number;

    return (
        <span
            onPointerEnter={() => properties.lighting.onLight(properties.number)}
            onPointerLeave={() => properties.lighting.onLight(undefined)}
            {...stylex.attrs(
                styles.spot,
                properties.isBlock === true && styles.block,
                properties.isOpen ? styles.joined : styles.seamed,
                isLit() && styles.lit,
            )}
        >
            <span {...stylex.attrs(styles.number, isLit() && styles.numberLit)}>
                {numberOf(properties.number)}
            </span>
            {properties.children}
        </span>
    );
}

/** The spot styles. */
const styles = stylex.create({
    spot: {
        alignItems: "center",
        borderRadius: "6px",
        cursor: "default",
        display: "inline-flex",
        outlineColor: "transparent",
        outlineOffset: "2px",
        outlineWidth: "1px",

        position: "relative",
        transitionDuration: "200ms",
        transitionProperty: "outline-color, background-color",
    },
    block: {
        display: "block",
    },
    seamed: {
        outlineStyle: "dashed",
    },
    joined: {
        outlineStyle: "solid",
    },
    lit: {
        backgroundColor: "rgb(255 121 46 / 10%)",
        outlineColor: tokens.signal,
    },
    number: {
        alignItems: "center",
        backgroundColor: color.card,
        borderColor: tokens.rule,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.mutedForeground,
        display: "inline-flex",
        fontFamily: tokens.monoFont,
        fontSize: "0.5625rem",
        height: "1rem",
        justifyContent: "center",
        minWidth: "1rem",
        right: "-0.75rem",
        paddingInline: "0.1875rem",
        position: "absolute",
        top: "-0.625rem",
        zIndex: 1,
    },
    numberLit: {
        backgroundColor: tokens.signal,
        borderColor: tokens.signal,
        color: tokens.signalInk,
    },
});
