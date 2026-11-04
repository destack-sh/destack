import {
    DeclarationName,
    defineDeclarationKind,
    ModuleMetadata,
    type Package,
} from "@destack/package";
import {
    type ViewCategory,
    ViewDescription,
    type ViewPresentationPriority,
} from "@destack/package/manifest";
import type { ObjectType } from "@destack/object";
import type { PackageHandle } from "@destack/object";
import type { Component } from "solid-js";
import type { Environment } from "@destack/environment";
import { ViewError } from "../error/error.ts";

/** The view declaration kind, which view defines. */
export const VIEW_KIND = defineDeclarationKind("view", ViewDescription);

/** How a view renders: in the browser, prerendered at build, or per request on the server. */
export type ViewRender =
    | {
          /** Rendered by the browser from the view's mount module. */
          readonly mode: "client";
      }
    | {
          /** Prerendered at build, its pages hydrating in the browser. */
          readonly mode: "static";
          /** The site's public origin, for absolute URLs in the sitemap, the feed and links. */
          readonly origin: string;
          /** How its pages hydrate: whole by default, or only their `islands/` components. */
          readonly hydrate?: "page" | "islands";
          /** Load the page shell wrapping the component, the view runtime's when absent. */
          readonly document?: () => Promise<{ readonly default: Component }>;
          /** Load the module of its paths, not-found path and endpoints, on the server. */
          readonly pages: () => Promise<StaticPages>;
      }
    | {
          /** Rendered per request by the package's workload, as the visitor. */
          readonly mode: "server";
      };

/** A static view's pages module, as Astro's getStaticPaths and static endpoints. */
export interface StaticPages {
    /** The paths prerendered below the view's base. */
    readonly paths: readonly string[];
    /** The path rendering the not-found page, published as 404.html, none when absent. */
    readonly notFound?: string;
    /** The files published beside the pages, each answering its path's request. */
    readonly endpoints?: Readonly<Record<string, Endpoint>>;
}

/** Answer a request for one of a static view's files, such as its sitemap. */
export type Endpoint = (context: EndpointContext) => Response | Promise<Response>;

/** What an endpoint answers with: the request, the site's origin and the environment it runs in. */
export interface EndpointContext {
    /** The request. */
    readonly request: Request;
    /** The site's public origin. */
    readonly origin: string;
    /** The environment the endpoint runs in. */
    readonly environment: Environment;
}

/** A view as its package defines it. */
export interface ViewDefinition {
    /** The name, unique among the package's views. */
    readonly name: string;
    /** The object types the view reads and changes, wherever its context opens them. */
    readonly objects?: readonly ObjectType[];
    /** The object types the view presents, and how strongly. */
    readonly presents?: readonly Presentation[];
    /** The roles clients open the view in, such as a person's start view. */
    readonly categories?: readonly ViewCategory[];
    /** The packages the view calls as its user, such as observability. */
    readonly packages?: readonly PackageHandle[];
    /** Load the module whose default export is the root component. */
    readonly component: () => Promise<{ readonly default: Component }>;
    /** How the view renders, in the browser when absent. */
    readonly render?: ViewRender;
}

/** An object type a view presents: the one it opens by default, or one offered beside it. */
export interface Presentation {
    /** The presented object type, among the view's objects. */
    readonly object: ObjectType;
    /** How strongly the view presents it. */
    readonly priority: ViewPresentationPriority;
}

/** A view a package declares, acting under its user's grant to the package's installation. */
export class View {
    /** The declaration kind of views, by its package and name, as builds index them. */
    static get kind() {
        return { kindPackageId: import.meta.destack.package.id, kind: "view" } as const;
    }

    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The object types the view reads and changes, wherever its context opens them. */
    readonly objects: readonly ObjectType[];
    /** The object types the view presents. */
    readonly presents: readonly Presentation[];
    /** The roles clients open the view in. */
    readonly categories: readonly ViewCategory[];
    /** The packages the view calls as its user. */
    readonly packages: readonly PackageHandle[];
    /** Load the module whose default export is the root component. */
    readonly component: ViewDefinition["component"];
    /** How the view renders. */
    readonly render: ViewRender;

    /** Create a view from its package and definition. */
    constructor(owner: Package, definition: ViewDefinition) {
        // copy the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.objects = definition.objects ?? [];
        this.presents = definition.presents ?? [];
        this.categories = definition.categories ?? [];
        this.packages = definition.packages ?? [];
        this.component = definition.component;
        this.render = definition.render ?? { mode: "client" };
    }

    /** List the object types the view opens in scopes of a type, with those living anywhere. */
    opens(lives: ObjectType): readonly ObjectType[] {
        return this.objects.filter(
            (object) =>
                object.scopes.length === 0 || object.scopes.some((each) => each.same(lives)),
        );
    }
}

/** Declare a view. */
export function defineView(definition: ViewDefinition, module?: ModuleMetadata): View {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineView").package;
    DeclarationName.parse(definition.name);

    // require each presented object type to be one the view opens
    const opened = definition.objects ?? [];
    const unpresentable = (definition.presents ?? []).find(
        ({ object }) => !opened.some((each) => each.same(object)),
    );
    if (unpresentable !== undefined) {
        throw ViewError.of(
            "PRESENTED_TYPE_NOT_OPENED",
            `view ${definition.name} presents ${unpresentable.object.name} but does not open ${unpresentable.object.name} objects`,
        );
    }

    return new View(owner, definition);
}
