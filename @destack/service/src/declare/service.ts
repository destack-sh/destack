import { DeclarationName, declaringModule, Package, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import type { ServiceRouter } from "../service/service.ts";

/** A declared HTTP service. */
export interface Service<Router extends ServiceRouter = ServiceRouter> extends Declaration {
    /** The declaration format version. */
    readonly version: 1;
    /** The service transport. */
    readonly protocol: "http";
    /** The service's procedures. */
    readonly router: Router;
    /** The declarations deriving procedures, by route name. */
    readonly objects: Readonly<Record<string, Routed>>;
}

/** A declaration that derives procedures, such as an object type. */
export interface Routed {
    /** The declaration's own procedures. */
    readonly procedures: ServiceRouter;
    /** The procedures every declaration of its kind shares. */
    readonly shared: ServiceRouter;
}

/** The procedures each kind of routed declaration derives. */
export interface RoutedProcedures<_Declaration> {}

/** The procedures a routed declaration derives. */
export type ProceduresOf<Declaration> =
    RoutedProcedures<Declaration>[keyof RoutedProcedures<Declaration>];

/** A service's procedures and routed declarations. */
export type ServiceInput = { readonly objects?: Readonly<Record<string, Routed>> } & {
    readonly [Name: string]: ServiceRouter | Readonly<Record<string, Routed>> | undefined;
};

/** The router a service input declares. */
export type ServiceRoutes<Input extends ServiceInput> = Extract<
    Omit<Input, "objects"> &
        (Input["objects"] extends Readonly<Record<string, Routed>>
            ? {
                  readonly [Name in keyof Input["objects"]]: ProceduresOf<Input["objects"][Name]>;
              } & Input["objects"][keyof Input["objects"]]["shared"]
            : unknown),
    ServiceRouter
>;

/** Declare an HTTP service. */
export function defineService<const Input extends ServiceInput>(
    name: string,
    input: Input,
    module?: ModuleMetadata,
): Service<ServiceRoutes<Input>> {
    // stamp the declaring package
    const owner = Package.parse(declaringModule(module, "defineService").package);

    // route each declaration under its name and the shared procedures once
    const { objects = {}, ...router } = input as ServiceInput;
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
    Object.assign(derived, shared);

    return Object.freeze({
        package: owner,
        name: DeclarationName.parse(name),
        version: 1,
        protocol: "http",
        router: derived as ServiceRoutes<Input>,
        objects,
    });
}
