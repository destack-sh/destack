import * as style from "@destack/style";
import { type Accessor, createSignal, type JSX } from "@destack/view";
import { IconBodies, type IconName, type IconWeight } from "@destack/icon";

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

/** The styles of an icon: a fixed-size graphic that a flex container never shrinks. */
const styles = style.create({
    icon: { flexShrink: 0 },
});

/** Draw a Phosphor icon inline in the current text color. */
export function Icon(properties: IconProperties): JSX.Element {
    // draw the body of the passed bodies, which the build passes for a literal name
    const body = (): string => {
        if (properties.icon === undefined) {
            throw new TypeError(`icon without bodies: '${properties.name ?? "unnamed"}'`);
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
            {...style.attrs(styles.icon, properties.xstyle)}
        />
    );
}

/** The bodies of an icon whose module has not arrived yet. */
const EMPTY: IconBodies = { thin: "", light: "", regular: "", bold: "", fill: "", duotone: "" };

/** The bodies of each requested icon, undefined until its module loads. */
const icons = new Map<IconName, Accessor<IconBodies | undefined>>();

/** The properties of an icon chosen at run time. */
export interface LazyIconProperties extends Omit<IconProperties, "name" | "icon"> {
    /** The Phosphor icon to draw. */
    readonly name: IconName;
}

/** Draw a Phosphor icon chosen at run time, empty at its final size until its module loads. */
export function LazyIcon(properties: LazyIconProperties): JSX.Element {
    return <Icon {...properties} icon={readIcon(properties.name) ?? EMPTY} />;
}

/** Read an icon's bodies, loading its module on first use. */
function readIcon(name: IconName): IconBodies | undefined {
    // reuse the icon once requested
    const existing = icons.get(name);
    if (existing !== undefined) {
        return existing();
    }

    // track the icon and fill it when its module arrives
    const [bodies, setBodies] = createSignal<IconBodies | undefined>(undefined);
    icons.set(name, bodies);
    void IconBodies.load(name).then((loaded) => setBodies(() => loaded));

    return bodies();
}
