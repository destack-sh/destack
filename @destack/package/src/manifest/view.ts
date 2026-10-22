import { schema, Text } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { PackagePath } from "../file/file.ts";
import { Sandbox } from "../definition/sandbox.ts";

/** The presentation priorities, the strongest first. */
const PRESENTATION_PRIORITIES = ["default", "option"] as const;

/** How strongly a view presents an object type: its default view, or one offered beside it. */
export const ViewPresentationPriority = Object.assign(schema.enum(PRESENTATION_PRIORITIES), {
    /** Compare two priorities, ordering the stronger first. */
    compare(left: ViewPresentationPriority, right: ViewPresentationPriority): number {
        return PRESENTATION_PRIORITIES.indexOf(left) - PRESENTATION_PRIORITIES.indexOf(right);
    },
});
/** How strongly a view presents an object type. */
export type ViewPresentationPriority = schema.Infer<typeof ViewPresentationPriority>;

/** An object type a view presents, as its declaration describes it. */
export const ViewPresentation = schema.object({
    /** The package declaring the object type. */
    packageId: PackageId,
    /** The object type. */
    type: schema.string().min(1),
    /** How strongly the view presents it. */
    priority: ViewPresentationPriority,
});
/** An object type a view presents, as its declaration describes it. */
export type ViewPresentation = schema.Infer<typeof ViewPresentation>;

/** A view a browser output compiles, as manifests describe it. */
const viewDescription = schema
    .object({
        /** The emitted chunk mounting the view. */
        entrypoint: PackagePath,
        /** The browser features the view may use beyond the bare sandbox. */
        sandbox: Sandbox,
        /** The object types the view presents, so opening an object picks its view. */
        presents: schema.array(ViewPresentation),
        /** The packages the view calls as its person. */
        packages: schema.array(PackageId),
    })
    .strict();

/** A view a browser output compiles, as manifests describe it. */
export const ViewDescription = Object.assign(viewDescription, {
    /** A view as its declaration describes it: its name, presented types and called packages. */
    declaration: viewDescription
        .pick({ presents: true, packages: true })
        .extend({ name: schema.string().min(1) }),

    /** List the views a build's outputs compile, by name. */
    list(manifest: {
        readonly outputs: Readonly<
            Record<string, { readonly views: Readonly<Record<string, ViewDescription>> }>
        >;
    }): Record<string, ViewDescription> {
        // name each view once across the outputs, refusing a name two outputs compile
        const views: Record<string, ViewDescription> = {};
        for (const [output, compiled] of Object.entries(manifest.outputs)) {
            for (const [name, view] of Object.entries(compiled.views)) {
                if (Object.hasOwn(views, name)) {
                    throw new TypeError(
                        `output ${output} compiles view ${name} another output does`,
                    );
                }
                views[name] = view;
            }
        }

        return views;
    },

    /** List view names by rank: views of no type first, then by strength and name. */
    rank(views: Readonly<Record<string, Pick<ViewDescription, "presents">>>): string[] {
        return Object.entries(views)
            .map(([name, view]) => ({ name, strength: strength(view) }))
            .toSorted(
                (left, right) =>
                    left.strength - right.strength || Text.compare(left.name, right.name),
            )
            .map((entry) => entry.name);
    },
});

/** A view a browser output compiles, as manifests describe it. */
export type ViewDescription = schema.Infer<typeof viewDescription>;

/** A call of an object type's method as a command the shell offers. */
export const CommandDescription = schema.object({
    /** The command's title, shown in the command menu. */
    title: schema.string().min(1),
    /** The package declaring the object type. */
    packageId: PackageId,
    /** The object type the command calls. */
    type: schema.string().min(1),
    /** The method the command calls. */
    method: schema.string().min(1),
    /** The key combination running it, in VS Code's keybinding syntax. */
    keybinding: schema.string().min(1).exactOptional(),
});
/** A named call of an object type's method the shell offers, as its declaration describes it. */
export type CommandDescription = schema.Infer<typeof CommandDescription>;

/** Rank a view by its strongest presentation, first when it presents no type. */
function strength(view: Pick<ViewDescription, "presents">): number {
    return view.presents.length === 0
        ? -1
        : Math.min(
              ...view.presents.map((presentation) =>
                  PRESENTATION_PRIORITIES.indexOf(presentation.priority),
              ),
          );
}
