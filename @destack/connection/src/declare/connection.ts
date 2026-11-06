import { ModuleMetadata } from "@destack/package";
import {
    type Connector,
    defineResourceKind,
    EGRESS_PROVIDER,
    type ExternalClient,
    type ResourceBinding,
    ResourceDeclaration,
} from "@destack/resource";
import { schema } from "@destack/schema";

/** What a package needs of a connection: the API its calls go below and the scopes they use. */
export const ConnectionSpec = schema.object({
    /** The protected resource the credential is for, the API's base URL (RFC 8707). */
    resource: schema.url({ protocol: /^https$/u }),
    /** The scopes the calls need, as the authorization server names them. */
    scopes: schema.array(schema.string().min(1)).default([]),
});
/** What a package needs of a connection. */
export type ConnectionSpec = schema.Infer<typeof ConnectionSpec>;

/** The connection kind: an external account's credential, which a person or a stack supplies. */
export const ConnectionKind = defineResourceKind("connection", {
    spec: ConnectionSpec,
    management: "external",
});
/** A connection a package declares. */
export type ConnectionDescription = schema.Infer<typeof ConnectionKind.description>;

/** The connector calling a bound connection's API below its address on the host's egress. */
const connectionConnector: Connector<ExternalClient> = {
    code: EGRESS_PROVIDER,
    connect: async (binding: ResourceBinding, declaration, environment) => {
        // require the instance credential the egress identifies the call by
        const { credential } = binding;
        if (credential === undefined) {
            throw new TypeError(`connection ${declaration.name} has no instance credential`);
        }

        // call below the bound address through the environment's network
        const base = binding.reference.replace(/\/$/u, "");

        return {
            url: base,
            fetch: (path: string, init: RequestInit = {}) => {
                // send the instance credential below the bound address
                const headers = new Headers(init.headers);
                headers.set("authorization", `Bearer ${credential}`);

                return environment.network.fetch(
                    new Request(`${base}${path}`, { ...init, headers }),
                );
            },
            [Symbol.asyncDispose]: () => Promise.resolve(),
        };
    },
};

/** A package's declaration of a connection it calls an API through. */
export class ConnectionDeclaration extends ResourceDeclaration<
    ExternalClient,
    ConnectionDescription
> {
    /** The connector calling the bound connection's API, by provider code. */
    override get connectors(): Readonly<Record<string, Connector<ExternalClient>>> {
        return { [connectionConnector.code]: connectionConnector };
    }
}

/** Declare a connection a package calls an API through, bound by name at install. */
export function defineConnection(
    name: string,
    spec: schema.Input<typeof ConnectionSpec>,
    module?: ModuleMetadata,
): ConnectionDeclaration {
    const owner = ModuleMetadata.require(module, "defineConnection").package;

    return new ConnectionDeclaration(
        owner,
        ConnectionKind.description.parse({ name, kind: ConnectionKind.name, spec }),
    );
}
