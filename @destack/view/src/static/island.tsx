import type { JsonValue } from "@destack/schema";
import { type Component, createContext, Hydration, NoHydration, useContext } from "solid-js";
import { isServer, type JSX } from "@solidjs/web";
import type { PackageVersion } from "@destack/package";
import { ViewError } from "../error/error.ts";
import { useLocale } from "../page/locale.ts";
import { ViewRoot } from "../page/mount.ts";
import { useEnvironment } from "../page/context.ts";

/** A page whose islands alone hydrate: its package and the ids it gave its islands so far. */
export class IslandPage {
    /** The package rendering the page, whose telemetry its islands record through. */
    readonly package: PackageVersion;
    /** The islands rendered so far. */
    #count = 0;

    /** Create a page of a package's islands. */
    constructor(owner: PackageVersion) {
        this.package = owner;
    }

    /** Take the hydration id of the next island the page renders. */
    next(module: string): string {
        const index = this.#count;
        this.#count += 1;

        return `${module}#${index}`;
    }
}

/** The page whose islands alone hydrate, absent on a page that hydrates whole. */
export const IslandContext = createContext<IslandPage | undefined>(undefined);

/** The properties an island takes: JSON, and children the server renders into its slot. */
export type IslandProperties<Properties> = {
    readonly [Key in keyof Properties]: Key extends "children"
        ? JSX.Element | undefined
        : JsonValue | undefined;
};

/** Render a component as an island: itself on a page hydrating whole, else in a root of its own. */
export function island<Properties extends IslandProperties<Properties>>(
    Island: Component<Properties>,
    module: string,
): Component<Properties> {
    return (properties): JSX.Element => {
        // render the component as it is on a page hydrating whole, or as the browser hydrates its root
        const page = useContext(IslandContext);
        if (page === undefined || !isServer) {
            return <Island {...properties} />;
        }

        // render it in a root the browser hydrates alone, with its properties as JSON and its children in its slot
        const { children, ...data }: { readonly children?: JSX.Element } = properties;
        if (!isJson(data)) {
            throw ViewError.of(
                "ISLAND_PROPERTIES_INVALID",
                `island '${module}' is invalid: it takes JSON properties and children only`,
            );
        }
        const id = page.next(module);
        const hasChildren = children !== undefined;
        const slotted = hasChildren ? { children: <IslandSlot>{children}</IslandSlot> } : {};

        return (
            <div
                data-island={module}
                data-island-id={id}
                data-island-properties={JSON.stringify(data)}
                data-island-children={hasChildren ? "" : undefined}
                style={{ display: "contents" }}
            >
                <Hydration id={id}>
                    <ViewRoot
                        package={page.package}
                        environment={useEnvironment()}
                        localization={useLocale()}
                    >
                        <Island {...properties} {...slotted} />
                    </ViewRoot>
                </Hydration>
            </div>
        );
    };
}

/** Render an island's children as static HTML the browser keeps, as Astro's slots. */
export function IslandSlot(properties: { readonly children?: JSX.Element }): JSX.Element {
    // render the children unhydrated on the server, and claim their element alone in the browser
    return (
        <div data-island-slot style={{ display: "contents" }}>
            {isServer ? <NoHydration>{properties.children}</NoHydration> : undefined}
        </div>
    );
}

/** Report whether a value is JSON as `JSON.stringify` keeps it, dropping undefined properties. */
function isJson(value: unknown): value is JsonValue {
    // take primitives JSON keeps
    if (value === null || typeof value === "string" || typeof value === "boolean") {
        return true;
    } else if (typeof value === "number") {
        return Number.isFinite(value);
    }
    // take arrays and plain objects of JSON
    else if (Array.isArray(value)) {
        return value.every((each) => isJson(each));
    } else if (typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
        return Object.values(value).every((each) => each === undefined || isJson(each));
    }
    // refuse functions, symbols, undefined, bigints and other objects
    else {
        return false;
    }
}
