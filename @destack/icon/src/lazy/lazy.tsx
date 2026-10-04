import { createSignal, type Accessor } from "solid-js";
import type { JSX } from "@solidjs/web";
import type { IconName } from "../phosphor/name.ts";
import { Icon, type IconBodies, type IconProperties } from "../icon/icon.tsx";

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
    void loadIcon(name).then((loaded) => setBodies(() => loaded));

    return bodies();
}

/** Load an icon's module, which bundlers split into one chunk per icon. */
async function loadIcon(name: IconName): Promise<IconBodies> {
    // import the module the name selects and require its bodies
    const module: unknown = await import(`../phosphor/icon/${name}.ts`);
    if (!isIconModule(module)) {
        throw new TypeError(`icon module without bodies: ${name}`);
    }

    return module.default;
}

/** Check that a value is a generated icon module, which always exports its bodies by default. */
function isIconModule(value: unknown): value is { readonly default: IconBodies } {
    return (
        typeof value === "object" &&
        value !== null &&
        "default" in value &&
        typeof value.default === "object"
    );
}
