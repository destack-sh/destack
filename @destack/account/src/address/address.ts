import { Forwarded, HostAddress } from "@destack/environment";
import { type Identifier, present, schema } from "@destack/schema";
import { AccountHandle, HANDLE_LENGTH } from "./handle.ts";
import { MachineName } from "../object/machine.ts";
import { InstallationName, PackageMount } from "@destack/object";
import { Label } from "./label.ts";

/** The longest DNS label, which one wildcard certificate covers (RFC 1035 2.3.4). */
const LABEL_LENGTH = 63;

/** The longest space or branch name, sized with `HANDLE_LENGTH` to fit one label. */
export const SPACE_NAME_LENGTH = 20;

/** The hexadecimal characters of a label digest, 40 bits that tell one space's branches apart. */
const DIGEST_LENGTH = 10;

/** The pattern of a label digest. */
const DIGEST_PATTERN = new RegExp(`^[0-9a-f]{${DIGEST_LENGTH}}$`, "u");

/** The offset basis of 64-bit FNV-1a. */
const FNV_OFFSET_BASIS = 0xcbf29ce484222325n;

/** The prime of 64-bit FNV-1a. */
const FNV_PRIME = 0x100000001b3n;

/** The mask truncating an FNV-1a hash to 64 bits. */
const FNV_MASK = 0xffffffffffffffffn;

/** The hexadecimal characters of a 64-bit hash. */
const FNV_HEX_LENGTH = 16;

/** The separator between the names in a label, as in `notes--personal--florian`. */
const NAME_SEPARATOR = "--";

/** The separator between a branch and its space in a label. */
const BRANCH_SEPARATOR = "---";

/** The first name of a space's S3 label, before its identifier. */
const S3_NAME = "s3";

/** The pattern of a name separator: a hyphen pair with no third hyphen beside it. */
const NAME_SEPARATOR_PATTERN = /(?<!-)--(?!-)/u;

/** The identifier schema of spaces. */
const SPACE_IDENTIFIER = schema.identifier("space");

/** The parent domains of view origins and machine addresses. */
export interface Domains {
    /** The domain of spaces' views, such as `destack.space`. */
    readonly space: string;
    /** The domain of machines, such as `destack.computer`. */
    readonly machine: string;
}

/** The domains the platform serves itself, reserved from account claims and their subdomains. */
export const DOMAINS = {
    /** The site. */
    site: "destack.sh",
    /** The universe's issuer, the universe's origin. */
    universe: "destack.app",
    /** The spaces' views. */
    space: "destack.space",
    /** The machines. */
    machine: "destack.computer",
} as const;

/** The domain installation origins answer below on a loopback machine, as browsers resolve it. */
export const LOOPBACK_DOMAIN = "localhost";

/** The path of the universe's issuer below its origin, the account's oauth adapter. */
export const OAUTH_PATH = "/oauth";

/** The universe's origin: its issuer below its path, and its packages below their mounts. */
export const UniverseOrigin = {
    /** Build the issuer URL of the universe at an origin, such as `https://destack.app/oauth`. */
    issuer(origin: string): string {
        return `${new URL(origin).origin}${OAUTH_PATH}`;
    },

    /** Read what a URL addresses at the universe's origin: its issuer or a package. */
    of(url: string, origin: string): "issuer" | "package" | undefined {
        const { origin: requested, pathname } = new URL(url);

        return requested === new URL(origin).origin ? UniverseOrigin.path(pathname) : undefined;
    },

    /** Read what a path addresses below the universe's origin: its issuer or a package. */
    path(pathname: string): "issuer" | "package" | undefined {
        // read the issuer's path, else a package's mount
        if (pathname === OAUTH_PATH || pathname.startsWith(`${OAUTH_PATH}/`)) {
            return "issuer";
        } else if (PackageMount.parse(pathname) !== undefined) {
            return "package";
        }

        return undefined;
    },
};

/** A machine of an account: `<machine>--<handle>.destack.computer`. */
export interface MachineAddress {
    /** The machine's name within its account. */
    readonly machine: string;
    /** The handle of the account the machine belongs to. */
    readonly handle: string;
}

/** Read machine addresses from server names. */
export const MachineAddress = {
    /** Read the machine address of a server name under a domain, absent for any other name. */
    parse(serverName: string, domain: string): MachineAddress | undefined {
        // take the two names of the label below the domain
        const label = labelBelow(serverName, domain);
        const [machine, handle, ...rest] = label?.split(NAME_SEPARATOR_PATTERN) ?? [];
        const isValid =
            machine !== undefined &&
            handle !== undefined &&
            rest.length === 0 &&
            MachineName.safeParse(machine).success &&
            AccountHandle.safeParse(handle).success;

        return isValid ? { machine, handle } : undefined;
    },

    /** Write the server name of a machine address under a domain. */
    format(address: MachineAddress, domain: string): string {
        return `${address.machine}${NAME_SEPARATOR}${address.handle}.${domain}`;
    },
};

/** A space's name within its account: a label of at most 20 characters. */
export const SpaceName = Label.schema(SPACE_NAME_LENGTH);

/** A space's address: its name, then its account's handle, as in `notes--florian`. */
export const SpaceAddress = Object.assign(
    schema
        .string()
        .regex(
            /^[a-z0-9]+(?:-[a-z0-9]+)*--[a-z0-9]+(?:-[a-z0-9]+)*$/u,
            "addresses are a space name and a handle joined by --: lowercase letters, digits and single inner hyphens",
        )
        .regex(
            new RegExp(`^[a-z0-9-]{1,${SPACE_NAME_LENGTH}}--[a-z0-9-]{1,${HANDLE_LENGTH}}$`, "u"),
            `space names take at most ${SPACE_NAME_LENGTH} characters, handles ${HANDLE_LENGTH}`,
        )
        .regex(
            /^(?:[^x]|x[^n]|xn[^-]|xn-[^-])[\s\S]*(?:[^n]|[^x]n|[^-]xn|[^-]-xn)$/u,
            "labels other than xn, which starts punycode",
        ),
    {
        /** Write the address of a space origin, leaving out its branch. */
        of(origin: Pick<SpaceOrigin, "space" | "handle">): string {
            return `${origin.space}${NAME_SEPARATOR}${origin.handle}`;
        },

        /** Read the origin of the space of an address. */
        origin(address: SpaceAddress): SpaceOrigin {
            return present(
                readSpace(address.split(NAME_SEPARATOR_PATTERN)),
                `the space ${address}`,
            );
        },
    },
);
/** A space's address. */
export type SpaceAddress = schema.Infer<typeof SpaceAddress>;

/** The origin a space answers at. */
export interface SpaceOrigin {
    /** The branch as the label writes it: its name, or the digest of a name too long for it. */
    readonly branch?: string;
    /** The space's name within its account. */
    readonly space: string;
    /** The handle of the space's account. */
    readonly handle: string;
}

/** The origin an installation answers at, serving its views and its package. */
export type InstallationOrigin = SpaceOrigin &
    (
        | {
              /** The installation's name. */
              readonly name: string;
              /** Absent beside a name. */
              readonly digest?: never;
          }
        | {
              /** The digest of the name and branch of a label longer than 63 characters. */
              readonly digest: string;
              /** Absent beside a digest. */
              readonly name?: never;
              /** Absent beside a digest. */
              readonly branch?: never;
          }
    );

/** Read and write installation origins under a domain. */
export const InstallationOrigin = {
    /** Read the installation origin of a hostname under a domain, or of a bare one without it. */
    parse(hostname: string, domain?: string): InstallationOrigin | undefined {
        // split the label at its branch separator and read its space and handle
        const { before, names } = splitLabel(hostname, domain);
        const [first, ...after] = names;
        const space = readSpace(before === undefined ? after : names);
        const [name, branch, ...rest] = before?.split(NAME_SEPARATOR_PATTERN) ?? [];
        if (space === undefined) {
            return undefined;
        }

        // read the name of the space itself
        if (before === undefined) {
            const isValid = first !== undefined && InstallationName.safeParse(first).success;

            return isValid ? { name: first, ...space } : undefined;
        }
        // read the name and branch of a readable branch
        else if (branch !== undefined) {
            const isValid =
                rest.length === 0 &&
                InstallationName.safeParse(name).success &&
                SpaceName.safeParse(branch).success;

            return isValid && name !== undefined ? { name, branch, ...space } : undefined;
        }
        // read the digest of a name and branch too long for the label
        else {
            return DIGEST_PATTERN.test(before) ? { digest: before, ...space } : undefined;
        }
    },

    /** Write the hostname an installation origin answers at under a domain. */
    format(origin: InstallationOrigin, domain?: string): string {
        // write the label of the space itself, a branch or a branch digest
        const label =
            origin.digest !== undefined
                ? branchedLabel(origin.digest, origin)
                : origin.branch === undefined
                  ? [origin.name, origin.space, origin.handle].join(NAME_SEPARATOR)
                  : branchedLabel(`${origin.name}${NAME_SEPARATOR}${origin.branch}`, origin);

        return domain === undefined ? label : `${label}.${domain}`;
    },

    /** Write the URL an installation origin answers at wherever its space's machine answers. */
    url(origin: InstallationOrigin, machine: string): string {
        // keep a loopback machine's port below the loopback domain
        const url = new URL(machine);
        if (HostAddress.isLoopbackName(url.hostname)) {
            url.hostname = InstallationOrigin.format(origin, LOOPBACK_DOMAIN);

            return url.origin;
        }

        return `https://${InstallationOrigin.format(origin, DOMAINS.space)}`;
    },

    /** Read the installation origin of a request under a domain. */
    of(request: Request, domain: string): InstallationOrigin | undefined {
        return InstallationOrigin.parse(askedHostname(request), domain);
    },
};

/** Read and write space origins under a domain. */
export const SpaceOrigin = {
    /** Read the space origin of a hostname under a domain, or of a bare one without it. */
    parse(hostname: string, domain?: string): SpaceOrigin | undefined {
        // split the label at its branch separator and read its space and handle
        const { before, names } = splitLabel(hostname, domain);
        const space = readSpace(names);
        if (space === undefined) {
            return undefined;
        }

        // read the branch by its name or digest
        const isBranch = before === undefined || SpaceName.safeParse(before).success;

        return isBranch
            ? { ...(before === undefined ? {} : { branch: before }), ...space }
            : undefined;
    },

    /** Write the hostname a space origin answers at under a domain. */
    format(origin: SpaceOrigin, domain?: string): string {
        // write the label of the space itself or a branch
        const label =
            origin.branch === undefined
                ? `${origin.space}${NAME_SEPARATOR}${origin.handle}`
                : branchedLabel(origin.branch, origin);

        return domain === undefined ? label : `${label}.${domain}`;
    },
};

/** Read and write the origins of spaces' buckets over S3: `s3--<space>` below a domain. */
export const S3Origin = {
    /** Read the space of an S3 origin's host name under a domain, absent for any other name. */
    parse(hostname: string, domain: string): Identifier<"space"> | undefined {
        // take the identifier after the S3 name in the label below the domain
        const [name, space, ...rest] =
            labelBelow(hostname, domain)?.split(NAME_SEPARATOR_PATTERN) ?? [];
        const parsed = SPACE_IDENTIFIER.safeParse(space);

        return name === S3_NAME && rest.length === 0 && parsed.success ? parsed.data : undefined;
    },

    /** Write the host name of a space's S3 origin under a domain. */
    format(space: Identifier<"space">, domain: string): string {
        return `${S3_NAME}${NAME_SEPARATOR}${space}.${domain}`;
    },

    /** Read the space of a request's S3 origin under a domain, the forwarded host first. */
    of(request: Request, domain: string): Identifier<"space"> | undefined {
        return S3Origin.parse(askedHostname(request), domain);
    },
};

/** Read the host name a request's client asked for, without its port. */
function askedHostname(request: Request): string {
    return new URL(`http://${Forwarded.host(request)}`).hostname;
}

/** Write a branch's label. */
function branchedLabel(before: string, origin: SpaceOrigin): string {
    // use the readable label while it fits
    const after = `${BRANCH_SEPARATOR}${origin.space}${NAME_SEPARATOR}${origin.handle}`;
    const readable = `${before}${after}`;

    return readable.length <= LABEL_LENGTH ? readable : `${labelDigest(before)}${after}`;
}

/** Read a space's name and handle from the last two names of a label, absent for other names. */
function readSpace(names: readonly string[]): Pick<SpaceOrigin, "space" | "handle"> | undefined {
    // take the last two names as a valid space and handle
    const [space, handle, ...rest] = names;
    const isValid =
        rest.length === 0 &&
        space !== undefined &&
        handle !== undefined &&
        SpaceName.safeParse(space).success &&
        AccountHandle.safeParse(handle).success;

    return isValid ? { space, handle } : undefined;
}

/** Split the label of a hostname below a domain. */
function splitLabel(
    hostname: string,
    domain: string | undefined,
): { readonly before?: string; readonly names: string[] } {
    // take the label below the domain
    const label = labelBelow(hostname, domain);
    if (label === undefined) {
        return { names: [] };
    }

    // split the label at the first branch separator
    const separator = label.indexOf(BRANCH_SEPARATOR);
    const after = separator < 0 ? label : label.slice(separator + BRANCH_SEPARATOR.length);
    const names = after.split(NAME_SEPARATOR_PATTERN);

    return separator < 0 ? { names } : { before: label.slice(0, separator), names };
}

/** Digest a label piece too long for 63 characters. */
function labelDigest(text: string): string {
    // fold each UTF-8 byte into the hash
    let hash = FNV_OFFSET_BASIS;
    for (const byte of new TextEncoder().encode(text)) {
        hash = ((hash ^ BigInt(byte)) * FNV_PRIME) & FNV_MASK;
    }

    return hash.toString(16).padStart(FNV_HEX_LENGTH, "0").slice(0, DIGEST_LENGTH);
}

/** Take the one label of a hostname below a domain, or a bare one without it. */
function labelBelow(hostname: string, domain: string | undefined): string | undefined {
    // take the label before the domain, refusing more than one
    const suffix = domain === undefined ? "" : `.${domain}`;
    const label = hostname.slice(0, hostname.length - suffix.length);
    const isLabel = hostname.endsWith(suffix) && !label.includes(".");

    return isLabel ? label : undefined;
}
