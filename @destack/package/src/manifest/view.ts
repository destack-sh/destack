import { defineSchema, schema, Text } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { PackagePath } from "../file/file.ts";
import { Capabilities } from "../definition/capability.ts";

/** The presentation priorities, the strongest first. */
const PRESENTATION_PRIORITIES = ["default", "option"] as const;

/** How strongly a view presents an object type: its default view, or one offered beside it. */
export const ViewPresentationPriority = Object.assign(
    defineSchema(schema.enum(PRESENTATION_PRIORITIES)),
    {
        /** Compare two priorities, ordering the stronger first. */
        compare(left: ViewPresentationPriority, right: ViewPresentationPriority): number {
            return PRESENTATION_PRIORITIES.indexOf(left) - PRESENTATION_PRIORITIES.indexOf(right);
        },
    },
);
/** How strongly a view presents an object type. */
export type ViewPresentationPriority = schema.Infer<typeof ViewPresentationPriority>;

/** An object type a view presents, as its declaration describes it. */
export const ViewPresentation = defineSchema(
    schema.object({
        /** The package declaring the object type. */
        packageId: PackageId,
        /** The object type. */
        type: schema.string().min(1),
        /** How strongly the view presents it. */
        priority: ViewPresentationPriority,
    }),
);
/** An object type a view presents, as its declaration describes it. */
export type ViewPresentation = schema.Infer<typeof ViewPresentation>;

/** A view a browser output compiles, as manifests describe it. */
export const ViewDescription = Object.assign(
    defineSchema(
        schema
            .object({
                /** The emitted chunk mounting the view. */
                entrypoint: PackagePath,
                /** The permissions the view requests by scope, as a `PermissionRequest`. */
                permissions: schema.record(schema.string(), schema.json()),
                /** The browser features the view may use, as its package declares them. */
                capabilities: Capabilities,
                /** The object types the view presents, so opening an object picks its view. */
                presents: schema.array(ViewPresentation),
                /** The packages the view calls as its person. */
                packages: schema.array(PackageId),
            })
            .strict(),
    ),
    {
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
    },
);

/** A view a browser output compiles, as manifests describe it. */
export type ViewDescription = schema.Infer<typeof ViewDescription>;

/** A call of an object type's method as a command the shell offers. */
export const CommandDescription = defineSchema(
    schema.object({
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
    }),
);
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
