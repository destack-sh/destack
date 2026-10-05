import { AccountHandle } from "@destack/account/object";
import { schema } from "@destack/schema";
import { HostName } from "@destack/account/object";

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
        const [host, handle, ...rest] = labels;
        const isValid =
            host !== undefined &&
            handle !== undefined &&
            rest.length === 0 &&
            HostName.safeParse(host).success &&
            AccountHandle.safeParse(handle).success;

        return isValid ? { host, handle } : undefined;
    },
    /** Write the server name of a host address under a domain, as in `laptop.florian.destack.computer`. */
    format(address: HostAddress, domain: string): string {
        return `${address.host}.${address.handle}.${domain}`;
    },
};

/** The separator between a branch and its space in a space label, as in `feature-x--personal`. */
const BRANCH_SEPARATOR = "--";

/** The path at which an installation's origin serves its package's service. */
export const SERVICE_PATH = "/.destack/service";

/** A space's address within its account: lowercase letters, digits and single hyphens between them, at most 63 characters. */
export const SpaceName = schema.string().regex(/^(?!-)(?!.*--)[a-z0-9-]{1,63}(?<!-)$/u);

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
    /** The handle of the space's account. */
    readonly handle: string;
}

/** Read and write installation origins under a domain, such as `destack.space`, or `localhost` on a device host. */
export const InstallationOrigin = {
    /** Read the installation origin of a hostname under a domain, or of a bare one without it. */
    parse(hostname: string, domain?: string): InstallationOrigin | undefined {
        // take the three labels below the domain
        const [alias, label, handle, ...rest] = labelsBelow(hostname, domain);
        if (alias === undefined || label === undefined || handle === undefined || rest.length > 0) {
            return undefined;
        }

        // read the space's label and refuse other labels
        const space = SpaceLabel.parse(label);
        const isValid =
            alias.length > 0 && space !== undefined && AccountHandle.safeParse(handle).success;

        return isValid ? { alias, ...space, handle } : undefined;
    },
    /** Write the hostname an installation origin answers at under a domain. */
    format(origin: InstallationOrigin, domain: string): string {
        return `${origin.alias}.${SpaceLabel.format(origin)}.${origin.handle}.${domain}`;
    },
};

/** The origin a space answers at, as in `personal.florian.destack.space` or `feature-x--personal.florian.destack.space`. */
export type SpaceOrigin = Omit<InstallationOrigin, "alias">;

/** Read and write space origins under a domain, such as `destack.space`, or `localhost` on a device host. */
export const SpaceOrigin = {
    /** Read the space origin of a hostname under a domain. */
    parse(hostname: string, domain: string): SpaceOrigin | undefined {
        // take the two labels below the domain
        const [label, handle, ...rest] = labelsBelow(hostname, domain);
        if (label === undefined || handle === undefined || rest.length > 0) {
            return undefined;
        }

        // read the space's label and refuse other labels
        const space = SpaceLabel.parse(label);
        const isValid = space !== undefined && AccountHandle.safeParse(handle).success;

        return isValid ? { ...space, handle } : undefined;
    },
    /** Write the hostname a space origin answers at under a domain. */
    format(origin: SpaceOrigin, domain: string): string {
        return `${SpaceLabel.format(origin)}.${origin.handle}.${domain}`;
    },
};

/** Read and write the label naming a space or one of its branches in an origin. */
const SpaceLabel = {
    /** Read a space label with its branch, absent for an invalid one. */
    parse(label: string): Pick<SpaceOrigin, "branch" | "space"> | undefined {
        // split at the first separator, and require valid names on both sides
        const separator = label.indexOf(BRANCH_SEPARATOR);
        const branch = separator > 0 ? label.slice(0, separator) : undefined;
        const space = separator > 0 ? label.slice(separator + BRANCH_SEPARATOR.length) : label;
        const isValid =
            SpaceName.safeParse(space).success &&
            (branch === undefined || SpaceName.safeParse(branch).success);

        return isValid ? { ...(branch === undefined ? {} : { branch }), space } : undefined;
    },
    /** Write the label of a space or one of its branches. */
    format(origin: Pick<SpaceOrigin, "branch" | "space">): string {
        return origin.branch === undefined
            ? origin.space
            : `${origin.branch}${BRANCH_SEPARATOR}${origin.space}`;
    },
};

/** Split the labels of a hostname below a domain, none for a hostname outside it. */
function labelsBelow(hostname: string, domain: string | undefined): string[] {
    const suffix = domain === undefined ? "" : `.${domain}`;

    return hostname.endsWith(suffix)
        ? hostname.slice(0, hostname.length - suffix.length).split(".")
        : [];
}
