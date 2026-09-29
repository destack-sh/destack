import { AccountHandle } from "@destack/account/object";
import { schema } from "@destack/schema";
import { HostName } from "../object/index.ts";

/** The parent domains of view origins and host addresses. */
export interface Domains {
    /** The domain of spaces' views, such as `destack.space`. */
    readonly space: string;
    /** The domain of hosts, such as `destack.computer`. */
    readonly host: string;
}

/** The parent domains of Destack's names. */
export const DOMAINS: Domains = { space: "destack.space", host: "destack.computer" };

/** A host of an account: `<host>.<handle>.destack.computer`. */
export interface HostAddress {
    /** The host's name within its account. */
    readonly host: string;
    /** The handle of the account the host belongs to. */
    readonly handle: string;
}

/** Read host addresses from server names. */
export const HostAddress = {
    /** Read the host address of a server name under a domain, absent for any other name. */
    parse(serverName: string, domain: string): HostAddress | undefined {
        // take the two labels below the domain
        const suffix = `.${domain}`;
        const labels = serverName.endsWith(suffix)
            ? serverName.slice(0, -suffix.length).split(".")
            : [];
        const [host, handle] = labels as [string, string];
        const isValid =
            labels.length === 2 &&
            HostName.safeParse(host).success &&
            AccountHandle.safeParse(handle).success;

        return isValid ? { host, handle } : undefined;
    },
};

/** The separator between a branch and its space in a space label, as in `feature-x--personal`. */
const BRANCH_SEPARATOR = "--";

/** The path at which an installation's origin serves its package's service. */
export const SERVICE_PATH = "/.destack/service";

/** A space's address within its account: lowercase letters, digits and single inner hyphens, at most 63 characters. */
export const SpaceName = schema.string().regex(/^(?!-)(?!.*--)[a-z0-9-]{1,63}(?<!-)$/);

/** A space's address: its name, then its account's handle, as in `notes.florian`. */
export const SpaceAddress = schema.templateLiteral([SpaceName, ".", AccountHandle]);
/** A space's address. */
export type SpaceAddress = schema.Infer<typeof SpaceAddress>;

/** The origin an installation answers at, serving its views and its service: `<alias>.<space label>.<handle>.<domain>`, with `<branch>--<space>` as the label of a branch. */
export interface InstallationOrigin {
    /** The installation's alias. */
    readonly alias: string;
    /** The branch of the space, absent for the space itself. */
    readonly branch?: string;
    /** The space's name within its account. */
    readonly space: string;
    /** The handle of the account holding the space. */
    readonly handle: string;
}

/** Read and write installation origins under a domain, such as `destack.space`, or `localhost` on a device host. */
export const InstallationOrigin = {
    /** Read the installation origin of a hostname under a domain, or of a bare one without it. */
    parse(hostname: string, domain?: string): InstallationOrigin | undefined {
        // take the three labels below the domain
        const suffix = domain === undefined ? "" : `.${domain}`;
        const labels = hostname.endsWith(suffix)
            ? hostname.slice(0, hostname.length - suffix.length).split(".")
            : [];
        if (labels.length !== 3) {
            return undefined;
        }
        const [alias, label, handle] = labels as [string, string, string];

        // split a branch from its space and refuse other labels
        const separator = label.indexOf(BRANCH_SEPARATOR);
        const branch = separator > 0 ? label.slice(0, separator) : undefined;
        const space = separator > 0 ? label.slice(separator + BRANCH_SEPARATOR.length) : label;
        const isValid =
            alias.length > 0 &&
            SpaceName.safeParse(space).success &&
            AccountHandle.safeParse(handle).success &&
            (branch === undefined || SpaceName.safeParse(branch).success);

        return isValid
            ? { alias, ...(branch === undefined ? {} : { branch }), space, handle }
            : undefined;
    },
    /** Write the hostname an installation origin answers at under a domain. */
    format(origin: InstallationOrigin, domain: string): string {
        const label =
            origin.branch === undefined
                ? origin.space
                : `${origin.branch}${BRANCH_SEPARATOR}${origin.space}`;

        return `${origin.alias}.${label}.${origin.handle}.${domain}`;
    },
};
