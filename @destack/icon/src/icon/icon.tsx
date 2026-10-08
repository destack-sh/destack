import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import type { IconName } from "../phosphor/icons.ts";

/** The stroke weight or style an icon is drawn in. */
export type IconWeight = "thin" | "light" | "regular" | "bold" | "fill" | "duotone";

/** The SVG body of one icon in each weight. */
export type IconBodies = Readonly<Record<IconWeight, string>>;

/** The properties of an icon. */
export interface IconProperties {
    /** The Phosphor icon to draw, written as a literal so the build passes its bodies as `icon`. */
    readonly name?: IconName;
    /** The bodies to draw, imported from `@destack/icon/phosphor/<name>`. */
    readonly icon?: IconBodies;
    /** The weight to draw the icon in, regular by default. */
    readonly weight?: IconWeight;
    /** The width and height as a CSS length or user units, 1em by default. */
    readonly size?: number | string;
    /** The accessible name, which exposes the icon as an image instead of hiding it. */
    readonly label?: string;
    /** The StyleX styles applied to the icon, such as its size or margins. */
    readonly xstyle?: style.Styles;
}

/** Draw a Phosphor icon inline in the current text color. */
export function Icon(properties: IconProperties): JSX.Element {
    // draw the body of the passed bodies, which the build passes for a literal name
    const body = (): string => {
        if (properties.icon === undefined) {
            throw new TypeError(`icon without bodies: ${properties.name ?? "unnamed"}`);
        }

        return properties.icon[properties.weight ?? "regular"];
    };

    return (
        <svg
            viewBox="0 0 256 256"
            fill="currentColor"
            width={properties.size ?? "1em"}
            height={properties.size ?? "1em"}
            role={properties.label === undefined ? undefined : "img"}
            aria-label={properties.label}
            aria-hidden={properties.label === undefined ? "true" : undefined}
            data-slot="icon"
            innerHTML={body()}
            {...style.attrs(properties.xstyle)}
        />
    );
}
