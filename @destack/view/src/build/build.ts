import type {
    BuildExtension,
    Compilation,
    DeclarationModule,
    Plugin,
} from "@destack/package/build";
import { BROWSER_SANDBOX, Sandbox } from "@destack/package";
import { CommandDescription, ViewDescription } from "@destack/package/manifest";
import { destackTheme } from "@destack/theme/declare";
import { SolidApplication } from "./application.ts";
import { fontPlugin } from "./font.ts";
import { imagePlugin } from "./image.ts";
import { VIEW_PACKAGE, viewPlugins } from "./plugin.ts";
import { ViewError } from "../error/error.ts";

/** The prefix of the generated modules mounting each view. */
const PREFIX = "virtual:@destack/view/view/";

/** Compile Solid components wherever modules compile. */
export const viewExtension: BuildExtension = {
    transform(context) {
        // compile Solid, as the output's application when it has one
        const application = context.options[VIEW_PACKAGE];

        return [
            ...viewPlugins(
                context.server,
                application === undefined ? undefined : SolidApplication.parse(application),
            ),
            imagePlugin(context.environment, context.cache),
            fontPlugin(context.environment.files),
        ];
    },
    compile(compilation) {
        // emit one browser entry per view
        const views = compilation.runtime === "browser" ? locate(compilation) : [];
        for (const view of views) {
            compilation.entry(`./view/${view.name}`, `${PREFIX}${view.name}`);
        }

        return [mountPlugin(views, compilation)];
    },
    describe(compilation, compiled) {
        // describe each view by its emitted chunk, browser abilities and presented types
        const declared = compilation.sandbox;
        const sandbox = Sandbox.select(
            declared,
            BROWSER_SANDBOX.filter((name) => declared[name] !== undefined),
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
                sandbox,
                presents: view.presents,
                categories: view.categories,
                packages: view.packages,
            };
        }

        return { views };
    },
};

/** Refuse a package with views that declares more than one theme. */
function requireOneTheme(compilation: Compilation, views: number): void {
    const themes = compilation.declarations.filter(
        (declaration) =>
            declaration.kind === "theme" &&
            declaration.symbol.package.id === compilation.package.id,
    );
    if (views > 0 && themes.length > 1) {
        throw ViewError.of(
            "MULTIPLE_THEMES",
            `a package with views declares at most one theme: ${themes.map((theme) => theme.name).join(", ")}`,
        );
    }
}

/** A view the package declares, with the module exporting it. */
interface LocatedView extends DeclarationModule {
    /** The view's name. */
    readonly name: string;
    /** The object types the view presents. */
    readonly presents: ViewDescription["presents"];
    /** The roles clients open the view in. */
    readonly categories: ViewDescription["categories"];
    /** The packages the view calls. */
    readonly packages: ViewDescription["packages"];
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
            throw ViewError.of("VIEW_DUPLICATE", `duplicate view: ${declaration.name}`);
        }

        // keep the module exporting the view, the types it presents, its roles and the packages it calls
        const { description } = declaration;
        const presents = ViewDescription.shape.presents.parse(description["presents"]);
        const categories = ViewDescription.shape.categories.parse(description["categories"]);
        const packages = ViewDescription.shape.packages.parse(description["packages"]);
        views.push({
            name: declaration.name,
            ...compilation.locate(declaration),
            presents,
            categories,
            packages,
        });
    }

    return views;
}

/** Load the generated module mounting each view with its package's theme and commands. */
function mountPlugin(views: readonly LocatedView[], compilation: Compilation): Plugin {
    // read the theme and the commands with their key combinations the package declares
    const own = compilation.declarations.filter(
        (declaration) => declaration.symbol.package.id === compilation.package.id,
    );
    const theme = own.find(
        (declaration) =>
            declaration.kind === "theme" && declaration.package.id === destackTheme.package.id,
    )?.description;
    const commands = own
        .filter(
            (declaration) =>
                declaration.kind === "command" && declaration.package.name === VIEW_PACKAGE,
        )
        .map((declaration) => {
            const { keybinding } = CommandDescription.parse(declaration.description);

            return {
                packageId: compilation.package.id,
                name: declaration.name,
                ...(keybinding === undefined ? {} : { keybinding }),
            };
        });
    const declared = JSON.stringify({ commands, ...(theme === undefined ? {} : { theme }) });

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
                        `import { BrowserEnvironment, mount } from "${VIEW_PACKAGE}/browser";`,
                        `import { ${view.export} as view } from ${JSON.stringify(view.file)};`,
                        `await mount(view, ${declared}, BrowserEnvironment.system);`,
                    ].join("\n")
                );
            },
        },
    };
}
