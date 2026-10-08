import * as style from "@destack/style";

import type { Entity } from "../past/card";
import { entities } from "../past/remix";
import { palette } from "../../palette.stylex";

/** The Home app, which the stack figure never shows as a card. */
const home: Entity = { label: "Home", icon: "notify", role: "App", tint: "#2f7d8c" };

/** Return the entity behind a name: the Home app, or one of the stack figure's entities. */
function entityOf(name: string): Entity {
    // read the entity, or refuse a name neither knows
    const entity = name === "home" ? home : entities[name];
    if (entity === undefined) {
        throw new RangeError(`no entity named ${name}`);
    }

    return entity;
}

/** Draw an entity's icon on a small tile in its own colour. */
export function Tile(properties: { name: string }) {
    const entity = () => entityOf(properties.name);

    return (
        <span
            style={{
                "background-color": entity().tint ?? palette.ink,
                color: entity().glyph ?? palette.cream,
            }}
            {...style.attrs(styles.tile)}
        >
            <span
                style={{ "mask-image": `url(/diagram/${entity().icon}.svg)` }}
                {...style.attrs(styles.glyph)}
            />
        </span>
    );
}

/** The tile styles. */
const styles = style.create({
    tile: {
        alignItems: "center",
        borderRadius: "5px",
        boxShadow: `inset 0 0 0 1px color-mix(in srgb, ${palette.ink} 18%, transparent)`,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.25rem",
        justifyContent: "center",
        width: "1.25rem",
    },
    glyph: {
        backgroundColor: "currentColor",
        height: "0.8125rem",
        maskPosition: "center",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "0.8125rem",
    },
});
