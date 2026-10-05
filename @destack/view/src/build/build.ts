import type {
    BuildExtension,
    Compilation,
    DeclarationModule,
    Plugin,
} from "@destack/package/build";
import { BROWSER_CAPABILITIES, Capabilities } from "@destack/package";
import { ViewDescription, ViewObjectType } from "@destack/package/view";
import { schema } from "@destack/schema";
import { SolidApplication } from "./application.ts";
import { VIEW_PACKAGE, viewPlugins } from "./plugin.ts";

/** The prefix of the generated modules mounting each view. */
const PREFIX = "virtual:@destack/view/view/";

/** Compile Solid components wherever modules compile, and mount each declared view from a separate browser chunk. */
export const viewExtension: BuildExtension = {
    transform(context) {
        // compile Solid, as the application an output names when it names one
        const application = context.options[VIEW_PACKAGE];

        return viewPlugins(
            context.server,
            application === undefined ? undefined : SolidApplication.parse(application),
        );
    },
    compile(compilation) {
        // emit one browser entry per view
        const views = compilation.runtime === "browser" ? locate(compilation) : [];
        for (const view of views) {
            compilation.entry(`./view/${view.name}`, `${PREFIX}${view.name}`);
        }

        return [mountPlugin(views)];
    },
    describe(compilation, compiled) {
        // describe each view by its emitted chunk, permissions, browser capabilities and presented types
        const declared = compilation.capabilities;
        const capabilities = Capabilities.select(
            declared,
            BROWSER_CAPABILITIES.filter((name) => declared[name] !== undefined),
        );
        const views: Record<string, ViewDescription> = {};
        const located = compilation.runtime === "browser" ? locate(compilation) : [];
        requireOneTheme(compilation, located.length);
        for (const view of located) {
            const entrypoint = compiled.exports[`./view/${view.name}`];
            if (entrypoint === undefined) {
                throw new TypeError(`missing view entrypoint: ${view.name}`);
            }
            views[view.name] = {
                entrypoint,
                permissions: view.permissions,
                capabilities,
                presents: view.presents,
                ...(view.home.length === 0 ? {} : { home: view.home }),
            };
        }

        return { views };
    },
};

/** Refuse a package with views that declares more than one theme, since its views take its theme. */
function requireOneTheme(compilation: Compilation, views: number): void {
    const themes = compilation.declarations.filter(
        (declaration) =>
            declaration.kind === "theme" &&
            declaration.symbol.package.id === compilation.package.id,
    );
    if (views > 0 && themes.length > 1) {
        throw new TypeError(
            `a package with views declares at most one theme: ${themes.map((theme) => theme.name).join(", ")}`,
        );
    }
}

/** A view the package declares, with the module exporting it. */
interface LocatedView extends DeclarationModule {
    /** The view's name. */
    readonly name: string;
    /** The permissions the view requests. */
    readonly permissions: ViewDescription["permissions"];
    /** The object types the view presents. */
    readonly presents: ViewDescription["presents"];
    /** The object types the view opens in the person's home. */
    readonly home: NonNullable<ViewDescription["home"]>;
}

/** Locate the views the package declares through the modules exporting them. */
function locate(compilation: Compilation): LocatedView[] {
    const views: LocatedView[] = [];
    for (const declaration of compilation.declarations) {
        // select views this package declares
        if (
            declaration.kind !== "view" ||
            declaration.package.name !== VIEW_PACKAGE ||
            declaration.symbol.package.id !== compilation.package.id
        ) {
            continue;
        }

        // refuse a name taken twice
        if (views.some((view) => view.name === declaration.name)) {
            throw new TypeError(`duplicate view: ${declaration.name}`);
        }

        // keep the module exporting the view, its permissions and the types it presents and opens in the home
        const { description } = declaration;
        const permissions = ViewDescription.shape.permissions.parse(description["permissions"]);
        const presents = ViewDescription.shape.presents.parse(description["presents"]);
        const home = schema.array(ViewObjectType).parse(description["home"] ?? []);
        views.push({
            name: declaration.name,
            ...compilation.locate(declaration),
            permissions,
            presents,
            home,
        });
    }

    return views;
}

/** Resolve and load the generated module mounting each view in the browser. */
function mountPlugin(views: readonly LocatedView[]): Plugin {
    return {
        name: VIEW_PACKAGE,
        resolveId: {
            filter: { id: new RegExp(`^${RegExp.escape(PREFIX)}`, "u") },
            handler: (id) => `\0${id}`,
        },
        load: {
            filter: { id: new RegExp(`^\\0${RegExp.escape(PREFIX)}`, "u") },
            handler(id) {
                // import the declaring module and mount its view
                const view = views.find((entry) => `\0${PREFIX}${entry.name}` === id);

                return (
                    view &&
                    [
                        `import { mount } from "${VIEW_PACKAGE}/browser";`,
                        `import { ${view.export} as view } from ${JSON.stringify(view.file)};`,
                        "await mount(view);",
                    ].join("\n")
                );
            },
        },
    };
}
