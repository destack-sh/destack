import type { IconName } from "../name/name.ts";

/** The stroke weight or style an icon is drawn in. */
export type IconWeight = "thin" | "light" | "regular" | "bold" | "fill" | "duotone";

/** The SVG body of one icon in each weight. */
export type IconBodies = Readonly<Record<IconWeight, string>>;

/** The SVG bodies of icons, and how to load one by name. */
export const IconBodies = {
    /** Load an icon's bodies from its module, which bundlers split into one chunk per icon. */
    async load(name: IconName): Promise<IconBodies> {
        // import the module the name selects and require its bodies
        const module: unknown = await import(`../phosphor/icon/${name}.ts`);
        if (!isIconModule(module)) {
            throw new TypeError(`icon module without bodies: '${name}'`);
        }

        return module.default;
    },
};

/** Check that a value is a generated icon module, which always exports its bodies by default. */
function isIconModule(value: unknown): value is { readonly default: IconBodies } {
    return (
        typeof value === "object" &&
        value !== null &&
        "default" in value &&
        typeof value.default === "object"
    );
}
