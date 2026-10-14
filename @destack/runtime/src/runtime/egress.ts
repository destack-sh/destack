import { DeclarationName } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { copyRequest } from "../request/request.ts";

/** The longest installation name, sized with an account handle to fit one DNS label. */
const INSTALLATION_NAME_LENGTH = 15;

/** The texts other than `xn`, which starts punycode labels, as a pattern without lookarounds. */
export const NOT_XN = /^(?:$|[^x]|x(?:$|[^n])|xn[\s\S])/u;

/** An installation's name within its space, unique there and fixed once installed. */
export const InstallationName = DeclarationName.max(INSTALLATION_NAME_LENGTH).regex(NOT_XN);

/** The path below which a host takes a workload's calls to addresses. */
const EGRESS_PATH = "/.destack/egress";

/** The separator between an installation's name and its space's identifier in its address. */
const INSTALLATION_SEPARATOR = "--";

/** An address followed by the path below it. */
const ADDRESSED = /^\/(@[^/]+\/[^/]+|[^/@][^/]*)(\/.*)?$/u;

/**
 * The paths at which a host takes its workloads' calls: `<egress>/<address>/<path>`.
 *
 * The address may be an installation its space binds, such as `notes`.
 * It may be an installation in a space by its name, as `<name>--<space>`.
 * It may be a package, such as `@destack/vault`.
 */
export const Egress = {
    /** The path of a host's egress on its loopback origin. */
    path: EGRESS_PATH,

    /** Write the URL at which a workload calls an address through its host's egress. */
    url(egress: string, address: string): string {
        return `${egress.replace(/\/$/u, "")}/${address}`;
    },

    /** Write the URL of a host path beside an egress, below the same prefix. */
    beside(egress: string, path: string): string {
        // keep the prefix above the egress path
        const url = new URL(egress);
        const prefix = url.pathname.endsWith(EGRESS_PATH)
            ? url.pathname.slice(0, -EGRESS_PATH.length)
            : "";
        url.pathname = `${prefix}${path}`;

        return url.href;
    },

    /** Write the address of an installation in a space, by its name there. */
    address(name: string, spaceId: string): string {
        return `${name}${INSTALLATION_SEPARATOR}${spaceId}`;
    },

    /** Read an installation's name and space from its address, absent for any other address. */
    installation(
        address: string,
    ): { readonly name: string; readonly spaceId: Identifier<"space"> } | undefined {
        // split the address into the name and the space's identifier
        const [name, spaceId, ...rest] = address.split(INSTALLATION_SEPARATOR);
        const named = InstallationName.safeParse(name);
        const space = schema.identifier("space").safeParse(spaceId);

        return named.success && space.success && rest.length === 0
            ? { name: named.data, spaceId: space.data }
            : undefined;
    },

    /** Split a request to a host's egress into the address and the request below it. */
    of(request: Request): { readonly address: string; readonly request: Request } | undefined {
        // require the egress path and an address below it
        const url = new URL(request.url);
        if (!url.pathname.startsWith(`${EGRESS_PATH}/`)) {
            return undefined;
        }
        const [, address, path] = ADDRESSED.exec(url.pathname.slice(EGRESS_PATH.length)) ?? [];
        if (address === undefined) {
            return undefined;
        }

        // keep the request below the address
        url.pathname = path ?? "/";

        return { address, request: copyRequest(request, {}, url.href) };
    },
};
