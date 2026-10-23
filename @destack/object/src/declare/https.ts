import {
    CONNECTION_PROVIDER,
    type Connector,
    defineResourceKind,
    type ResourceBinding,
    ResourceDeclaration,
} from "@destack/resource";
import { ModuleMetadata } from "@destack/package";
import { schema } from "@destack/schema";

/** An HTTPS API's specification: the URL its calls go below. */
export const HttpsSpec = schema.object({
    /** The API's base URL, such as https://api.github.com. */
    url: schema.url({ protocol: /^https$/u }),
});
/** An HTTPS API's specification. */
export type HttpsSpec = schema.Infer<typeof HttpsSpec>;

/** The HTTPS resource kind: an external API a host lends a connection's credential to. */
export const HttpsKind = defineResourceKind("https", { spec: HttpsSpec, management: "external" });
/** A named dependency on an HTTPS API. */
export type HttpsDescription = schema.Infer<typeof HttpsKind.description>;

/** A client of a bound HTTPS API. */
export interface HttpsClient {
    /** The bound address below the host's egress that calls go to. */
    readonly url: string;
    /** Call a path below the API's base URL. */
    readonly fetch: (path: string, init?: RequestInit) => Promise<Response>;
}

/** A dependency on an HTTPS API, called through the host's egress. */
export class HttpsBinding extends ResourceDeclaration<HttpsClient, HttpsDescription> {
    /** The connector of lent resources. */
    override get connectors(): { readonly connection: Connector<HttpsClient> } {
        return {
            connection: {
                code: CONNECTION_PROVIDER,
                connect: async (binding: ResourceBinding) => {
                    // require the instance credential the host's egress identifies the call by
                    const credential = binding.credential;
                    if (credential === undefined) {
                        throw new TypeError(`https binding ${this.name} has no credential`);
                    }

                    // call below the bound address, disposing nothing
                    const base = binding.reference.replace(/\/$/u, "");

                    return {
                        url: base,
                        fetch: (path: string, init: RequestInit = {}) => {
                            const headers = new Headers(init.headers);
                            headers.set("authorization", `Bearer ${credential}`);

                            return fetch(`${base}${path}`, { ...init, headers });
                        },
                        [Symbol.asyncDispose]: () => Promise.resolve(),
                    };
                },
            },
        };
    }
}

/** Declare a dependency on an HTTPS API. */
export function defineHttps(name: string, spec: HttpsSpec, module?: ModuleMetadata): HttpsBinding {
    const owner = ModuleMetadata.require(module, "defineHttps").package;

    return new HttpsBinding(
        owner,
        HttpsKind.description.parse({ name, kind: HttpsKind.name, spec }),
    );
}
