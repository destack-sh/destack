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

/** A permission a view requests on an object type. */
export const ViewPermission = defineSchema(
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
);
/** A permission a view requests on an object type. */
export type ViewPermission = schema.Infer<typeof ViewPermission>;

/** The scopes a view's permissions apply in, relative to its context. */
const VIEW_SCOPES = ["space", "home", "account"] as const;

/** A scope relative to a view's context, with a later fourth for objects the person picks, granted per object through a shell picker. */
export const ViewScope = defineSchema(schema.enum(VIEW_SCOPES));
/** A scope relative to a view's context: its space, the person's home, or the space's account. */
export type ViewScope = schema.Infer<typeof ViewScope>;

/** The permissions a view requests, by the scope they apply in. */
export const ViewPermissions = defineSchema(
    schema
        .object({
            /** The permissions in the space the view runs in. */
            space: schema.array(ViewPermission).exactOptional(),
            /** The permissions in the person's home. */
            home: schema.array(ViewPermission).exactOptional(),
            /** The permissions in the account of the view's space. */
            account: schema.array(ViewPermission).exactOptional(),
        })
        .strict(),
);
/** The permissions a view requests, by the scope they apply in. */
export type ViewPermissions = schema.Infer<typeof ViewPermissions>;

/** A view a browser output compiles, as manifests describe it. */
export const ViewDescription = Object.assign(
    defineSchema(
        schema
            .object({
                /** The emitted chunk mounting the view. */
                entrypoint: PackagePath,
                /** The permissions the view requests, by the scope they apply in. */
                permissions: ViewPermissions,
                /** The browser features the view may use, as its package declares them. */
                capabilities: Capabilities,
                /** The object types the view presents, so opening an object picks its view. */
                presents: schema.array(ViewPresentation),
                /** The packages of the platform services the view calls as its person. */
                services: schema.array(PackageId),
            })
            .strict(),
    ),
    {
        /** Grant a view's permissions in the scopes they name, dropping those of a scope the view's context lacks. */
        grants(
            view: { readonly permissions: ViewPermissions },
            scopes: {
                readonly space: string;
                readonly home: string | undefined;
                readonly account: string | undefined;
            },
        ): (ViewPermission & { readonly scope: string })[] {
            return ViewScope.options.flatMap((name) => {
                // grant the scope's permissions where the context names the scope
                const scope = scopes[name];

                return scope === undefined
                    ? []
                    : (view.permissions[name] ?? []).map((permission) => ({
                          ...permission,
                          scope,
                      }));
            });
        },

        /** List view names from the top-ranked: views presenting no type first, the rest by their strongest presentation and by name. */
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
