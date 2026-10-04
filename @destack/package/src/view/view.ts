import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { PackagePath } from "../file/file.ts";
import { Capabilities } from "../definition/capability.ts";

/** The presentation priorities, the strongest first. */
const PRESENTATION_PRIORITIES = ["default", "option"] as const;

/** How strongly a view presents an object type: the one it opens by default, or one offered beside it, as VS Code's custom editors prioritise theirs. */
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
                /** The permissions the view requests. */
                permissions: schema.array(
                    schema
                        .object({
                            /** The package declaring the permission. */
                            packageId: PackageId,
                            /** The object type. */
                            type: schema.string().min(1),
                            /** The permission on that type. */
                            name: schema.string().min(1),
                        })
                        .strict(),
                ),
                /** The browser features the view may use, as its package declares them. */
                capabilities: Capabilities,
                /** The object types the view presents, so opening an object picks its view. */
                presents: schema.array(ViewPresentation),
            })
            .strict(),
    ),
    {
        /** List view names from the top-ranked: views presenting no type, then by their strongest presentation, then by name. */
        rank(views: Readonly<Record<string, Pick<ViewDescription, "presents">>>): string[] {
            return Object.entries(views)
                .map(([name, view]) => ({ name, strength: strength(view) }))
                .toSorted(
                    (left, right) =>
                        left.strength - right.strength ||
                        Number(left.name > right.name) - Number(left.name < right.name),
                )
                .map((entry) => entry.name);
        },
    },
);

/** A view a browser output compiles, as manifests describe it. */
export type ViewDescription = schema.Infer<typeof ViewDescription>;

/** A named call of an object type's method the shell offers in its command menu, menus, shortcuts, the terminal and agents, as its declaration describes it. */
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

/** Rank a view by its strongest presentation, before every presentation when it presents no type. */
function strength(view: Pick<ViewDescription, "presents">): number {
    return view.presents.length === 0
        ? -1
        : Math.min(
              ...view.presents.map((presentation) =>
                  PRESENTATION_PRIORITIES.indexOf(presentation.priority),
              ),
          );
}
