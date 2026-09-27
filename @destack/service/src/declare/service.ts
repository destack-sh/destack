import { DeclarationName, declaringModule, Package, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import type { ServiceRouter } from "../service/service.ts";

/** A declared HTTP service and the procedures its workload implements. */
export interface Service<Router extends ServiceRouter = ServiceRouter> extends Declaration {
    /** The declaration format version. */
    readonly version: 1;
    /** The service transport. */
    readonly protocol: "http";
    /** Procedures implemented by the named service. */
    readonly router: Router;
    /** The declarations deriving procedures, such as object types, keyed by the name each routes under. */
    readonly objects: Readonly<Record<string, Routed>>;
}

/** A declaration that derives the procedures a service routes to it, such as an object type. */
export interface Routed {
    /** The declaration's own procedures, routed under its key. */
    readonly procedures: ServiceRouter;
    /** The procedures every declaration of its kind shares, routed once at the service's top level. */
    readonly shared: ServiceRouter;
}

/** The procedures each kind of routed declaration derives, by kind, which the packages declaring the kinds add to. */
export interface RoutedProcedures<_Declaration> {}

/** The procedures a routed declaration derives, as the package declaring its kind types them. */
export type ProceduresOf<Declaration> =
    RoutedProcedures<Declaration>[keyof RoutedProcedures<Declaration>];

/** A service's procedures and the declarations deriving more, keyed by the name each routes under. */
export type ServiceInput = { readonly objects?: Readonly<Record<string, Routed>> } & {
    readonly [Name: string]: ServiceRouter | Readonly<Record<string, Routed>> | undefined;
};

/** The router a service input declares: its own procedures, each declaration's, and their shared ones. */
export type ServiceRoutes<Input extends ServiceInput> = Extract<
    Omit<Input, "objects"> &
        (Input["objects"] extends Readonly<Record<string, Routed>>
            ? {
                  readonly [Name in keyof Input["objects"]]: ProceduresOf<Input["objects"][Name]>;
              } & Input["objects"][keyof Input["objects"]]["shared"]
            : unknown),
    ServiceRouter
>;

/** Declare an HTTP service, its procedures and the declarations deriving more, for workload routing. */
export function defineService<const Input extends ServiceInput>(
    name: string,
    input: Input,
    module?: ModuleMetadata,
): Service<ServiceRoutes<Input>> {
    // stamp the declaring package supplied by the module transform
    const owner = Package.parse(declaringModule(module, "defineService").package);

    // route each declaration under its own name, and their shared procedures once
    const { objects = {}, ...router } = input as ServiceInput;
    const derived: Record<string, unknown> = { ...router };
    for (const [key, routed] of Object.entries(objects)) {
        derived[key] = routed.procedures;
        Object.assign(derived, routed.shared);
    }

    return Object.freeze({
        package: owner,
        name: DeclarationName.parse(name),
        version: 1,
        protocol: "http",
        router: derived as ServiceRoutes<Input>,
        objects,
    });
}
