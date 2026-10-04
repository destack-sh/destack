import { DeclarationName, ModuleMetadata, Package } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { Version } from "@destack/schema";
import type { ServiceRouter } from "../service/service.ts";

/** A declared HTTP service. */
export interface Service<
    Router extends ServiceRouter = ServiceRouter,
    Objects extends Readonly<Record<string, Routed>> = Readonly<Record<string, Routed>>,
> extends Declaration {
    /** The oldest caller release the service serves, every release when absent. */
    readonly since?: Version;
    /** The service transport. */
    readonly protocol: "http";
    /** The service's procedures. */
    readonly router: Router;
    /** The declarations deriving procedures, by route name. */
    readonly objects: Objects;
}

/** A declaration that derives procedures, such as an object type. */
export interface Routed {
    /** The declaration's own procedures. */
    readonly procedures: ServiceRouter;
    /** The procedures every declaration of its kind shares. */
    readonly shared: ServiceRouter;
}

/** A service's procedures and routed declarations. */
export type ServiceInput = {
    readonly objects?: Readonly<Record<string, Routed>>;
    readonly since?: Version;
} & {
    readonly [Name: string]: ServiceRouter | Readonly<Record<string, Routed>> | Version | undefined;
};

/** The router a service input declares. */
export type ServiceRoutes<Input extends ServiceInput> = Extract<
    Omit<Input, "objects" | "since"> &
        (Input["objects"] extends Readonly<Record<string, Routed>>
            ? {
                  readonly [Name in keyof Input["objects"]]: Input["objects"][Name]["procedures"];
              } & Input["objects"][keyof Input["objects"]]["shared"]
            : unknown),
    ServiceRouter
>;

/** Declare an HTTP service routing the input's procedures, each routed declaration's under its name, and the shared ones. */
export function defineService<const Input extends ServiceInput>(
    name: string,
    input: Input,
    module?: ModuleMetadata,
): Service<
    ServiceRoutes<Input>,
    Input["objects"] extends Readonly<Record<string, Routed>> ? Input["objects"] : {}
>;
/**
 * Declare an HTTP service with the router routeObjects derives.
 *
 * @construct routeObjects routes each declaration under its own name and each object's procedures under its key, which is how ServiceRoutes maps the input.
 */
export function defineService(name: string, input: ServiceInput, module?: ModuleMetadata): unknown {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineService").package);

    // route each declaration under its name and the shared procedures once
    const { objects = {}, since, ...router } = input;
    const derived = routeObjects(name, router, objects);

    return Object.freeze({
        package: owner,
        name: DeclarationName.parse(name),
        ...(since === undefined ? {} : { since: Version.parse(since) }),
        protocol: "http",
        router: derived,
        objects,
    });
}

/** Route each declaration under its name beside a router, with the shared procedures once. */
function routeObjects(
    name: string,
    router: Readonly<Record<string, unknown>>,
    objects: Readonly<Record<string, Routed>>,
): Record<string, unknown> {
    // route each declaration under its name and collect the shared procedures
    const derived: Record<string, unknown> = { ...router };
    const shared: Record<string, unknown> = {};
    for (const [key, routed] of Object.entries(objects)) {
        derived[key] = routed.procedures;
        Object.assign(shared, routed.shared);
    }

    // refuse a name taken twice
    const taken = [...Object.keys(router), ...Object.keys(objects), ...Object.keys(shared)];
    const collision = taken.find((key, position) => taken.indexOf(key) !== position);
    if (collision !== undefined) {
        throw new TypeError(`service ${name} routes two procedures under ${collision}`);
    }

    return Object.assign(derived, shared);
}
