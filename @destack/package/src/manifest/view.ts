import { schema, Text } from "@destack/schema";
import { PackageId } from "../definition/package.ts";
import { PackagePath } from "../file/file.ts";
import { Sandbox } from "../definition/sandbox.ts";
import { PackageError } from "../error/index.ts";

/** The roles a client opens a view in: a start view, or a command palette over a window. */
const VIEW_CATEGORIES = ["home", "command"] as const;

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

/** A role a client opens a view in, as Android's intent categories mark a home screen. */
export const ViewCategory = schema.enum(VIEW_CATEGORIES);
/** A role a client opens a view in. */
export type ViewCategory = schema.Infer<typeof ViewCategory>;

/** How a build's manifest says a view renders. */
const viewRender = schema.discriminatedUnion("mode", [
    schema.object({
        /** Rendered by the browser from the view's mount module. */
        mode: schema.literal("client"),
    }),
    schema.object({
        /** Prerendered at build into its pages directory, its pages hydrating in the browser. */
        mode: schema.literal("static"),
    }),
    schema.object({
        /** Rendered per request by the package's workload, as the visitor. */
        mode: schema.literal("server"),
        /** The export of the package's server output rendering the view. */
        entrypoint: PackagePath,
    }),
]);

/** How a view renders: in the browser, prerendered at build, or on the server per request. */
export const ViewRender = Object.assign(viewRender, {
    /** How a view's declaration says it renders, before its build. */
    declaration: schema.discriminatedUnion("mode", [
        schema.object({
            /** Rendered by the browser. */
            mode: schema.literal("client"),
        }),
        schema.object({
            /** Prerendered at build. */
            mode: schema.literal("static"),
            /** The site's public origin. */
            origin: schema.url(),
            /** How its pages hydrate: whole, or only their islands. */
            hydrate: schema.enum(["page", "islands"]),
        }),
        schema.object({
            /** Rendered per request on the server. */
            mode: schema.literal("server"),
        }),
    ]),
});
/** How a view renders. */
export type ViewRender = schema.Infer<typeof ViewRender>;

/** A view a browser output compiles, as manifests describe it. */
const viewDescription = schema
    .object({
        /** The emitted chunk mounting the view. */
        entrypoint: PackagePath,
        /** The stylesheets the view's pages link, in order. */
        stylesheets: schema.array(PackagePath),
        /** The browser features the view may use beyond the bare sandbox. */
        sandbox: Sandbox,
        /** The object types the view presents, so opening an object picks its view. */
        presents: schema.array(ViewPresentation),
        /** The roles clients open the view in. */
        categories: schema.array(ViewCategory),
        /** The packages the view calls as its person. */
        packages: schema.array(PackageId),
        /** How the view renders. */
        render: ViewRender,
    })
    .strict();

/** A view a browser output compiles, as manifests describe it. */
export const ViewDescription = Object.assign(viewDescription, {
    /** A view as its declaration describes it: name, presented types, packages and render mode. */
    declaration: viewDescription.pick({ presents: true, categories: true, packages: true }).extend({
        name: schema.string().min(1),
        render: ViewRender.declaration,
    }),

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
                    throw PackageError.of(
                        "VIEW_DUPLICATE",
                        `output ${output} compiles view ${name}, which another output already compiles`,
                    );
                }
                views[name] = view;
            }
        }

        return views;
    },

    /** Read the directory of a static view's pages in the browser output describing it. */
    pages(output: { readonly directory: string }, view: string): string {
        return `${output.directory}/view/${view}`;
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
