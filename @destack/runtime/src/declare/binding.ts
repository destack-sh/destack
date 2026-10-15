import {
    type ResourceBinding,
    type Connector,
    defineResourceKind,
    ResourceDeclaration,
} from "@destack/resource";
import { ModuleMetadata, type Package, PackageId } from "@destack/package";
import { defineSchema, schema, Version } from "@destack/schema";
import { Exchange, type ExchangeContext } from "../authentication/exchange.ts";
import { createClient } from "../client/client.ts";
import type { Client, ProcedureRouter } from "../runtime/runtime.ts";
import type { PackageHandle } from "./package.ts";

/** The provider code of installation bindings, connecting over HTTP through the host's egress. */
export const INSTALLATION_PROVIDER = "http";

/** An installation binding's specification. */
export const InstallationBindingSpec = defineSchema(
    schema.object({
        /** The package. */
        package: PackageId,
        /**
         * The address an installation calls unless a stack binds another.
         *
         * An example is `@destack/message` for a package of the platform.
         */
        address: schema.string().min(1).exactOptional(),
    }),
);
/** An installation binding's specification. */
export type InstallationBindingSpec = schema.Infer<typeof InstallationBindingSpec>;

/** The state an installation binding requires of its callee: its caller's build release. */
export const InstallationBindingState = defineSchema(schema.object({ release: Version }));
/** The state an installation binding requires of its callee. */
export type InstallationBindingState = schema.Infer<typeof InstallationBindingState>;

/** The installation resource kind: typed clients of another package's installation. */
export const InstallationKind = defineResourceKind("installation", {
    spec: InstallationBindingSpec,
});
/** A named dependency on an installation of a package. */
export type InstallationBindingDescription = schema.Infer<typeof InstallationKind.description>;

/** A dependency on an installation of a package, bound to a typed client of its endpoint. */
export class InstallationBinding<
    Router extends ProcedureRouter = ProcedureRouter,
> extends ResourceDeclaration<Client<Router, ExchangeContext>, InstallationBindingDescription> {
    /** The package the client calls. */
    readonly handle: PackageHandle & { readonly router: Router };

    /** Create the binding. */
    constructor(
        owner: Package,
        description: InstallationBindingDescription,
        handle: PackageHandle & { readonly router: Router },
    ) {
        super(owner, description);
        this.handle = handle;
    }

    /** The HTTP connector. */
    override get connectors(): { readonly http: Connector<Client<Router, ExchangeContext>> } {
        return {
            http: {
                code: INSTALLATION_PROVIDER,
                connect: async (binding: ResourceBinding) => {
                    // require the credential the host lends the binding
                    const credential = binding.credential;
                    if (credential === undefined) {
                        throw new TypeError(`installation binding ${this.name} has no credential`);
                    }

                    // ask for an exchange for the subject in a call's context
                    const client = createClient<Router, ExchangeContext>(this.handle, {
                        url: binding.reference,
                        headers: (options) => ({
                            authorization: `Bearer ${credential}`,
                            ...Exchange.ask(options.context?.exchange),
                        }),
                    });

                    // dispose nothing
                    return Object.assign(client, {
                        [Symbol.asyncDispose]: () => Promise.resolve(),
                    });
                },
            },
        };
    }
}

/** Declare a dependency on an installation of a package. */
export function defineInstallationBinding<Router extends ProcedureRouter>(
    name: string,
    handle: PackageHandle & { readonly router: Router },
    options: {
        /** The address an installation calls unless a stack binds another. */
        readonly address?: string;
    } = {},
    module?: ModuleMetadata,
): InstallationBinding<Router> {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineInstallationBinding").package;
    const description = InstallationKind.description.parse({
        name,
        kind: InstallationKind.name,
        spec: {
            package: handle.id,
            ...(options.address === undefined ? {} : { address: options.address }),
        },
    });

    return new InstallationBinding(owner, description, handle);
}
