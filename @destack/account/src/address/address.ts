import { Forwarded, HostAddress } from "@destack/environment";
import { type Identifier, present, schema } from "@destack/schema";
import { AccountHandle, HANDLE_LENGTH } from "./handle.ts";
import { MachineName } from "../object/machine.ts";
import { HOME_APPLICATION, HOME_SPACE } from "../object/account.ts";
import { Hostname } from "../object/domain.ts";
import { InstallationName, PackageMount } from "@destack/object";
import { RepositoryName } from "@destack/package";
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

/** The domain of a loopback machine's installation origins, apart from a universe's own. */
export const LOOPBACK_DOMAIN = "space.localhost";

/** The path of the universe's issuer below its origin, the account's oauth mount. */
export const OAUTH_PATH = "/oauth";

/** The paths of the universe's pages people sign in, consent and approve devices on. */
export const UNIVERSE_PAGES = {
    signIn: "/sign-in",
    signUp: "/sign-up",
    consent: "/consent",
    verification: "/verification",
    apps: "/apps",
} as const;

/** A link at the universe's origin: an account's app or repository. */
export type UniverseLink =
    | {
          /** An app, at `/@<handle>/<space>/<app>`, redirecting to its installation. */
          readonly kind: "app";
          /** The installation the link opens. */
          readonly installation: InstallationOrigin;
      }
    | {
          /** A Git repository, at `/@<handle>/<name>.git`, forwarded to its registry. */
          readonly kind: "repository";
          /** The account's handle. */
          readonly handle: string;
          /** The repository's name within the account. */
          readonly name: RepositoryName;
          /** The Git path below the repository, such as `/info/refs`, or empty. */
          readonly path: string;
      };

/** A universe's address: its origin, and the parent domains of its spaces' and machines' names. */
export class UniverseOrigin {
    /** A universe's address as JSON: its origin and its domains. */
    static readonly schema = schema.object({
        /** The universe's origin, its issuer's, such as `https://destack.app`. */
        origin: schema.url(),
        /** The domains the universe serves beside its origin's. */
        domains: schema.object({
            /** The domain of spaces' views, such as `destack.space`. */
            space: Hostname,
            /** The domain of machines, such as `destack.computer`. */
            machine: Hostname,
            /** The domain of the universe's site, such as `destack.sh`. */
            site: Hostname,
        }),
    });

    /** The universe's origin, its issuer's, such as `https://destack.app`. */
    readonly origin: string;
    /** The domains the universe serves beside its origin's: its spaces', machines' and site's. */
    readonly domains: schema.Infer<typeof UniverseOrigin.schema>["domains"];

    /** Address a universe at its origin, with the domains its names answer below. */
    constructor(origin: string, domains: UniverseOrigin["domains"]) {
        this.origin = new URL(origin).origin;
        this.domains = domains;
    }

    /** Read a universe's address from its JSON. */
    static of(address: schema.Infer<typeof UniverseOrigin.schema>): UniverseOrigin {
        return new UniverseOrigin(address.origin, address.domains);
    }

    /** Build the issuer URL of the universe at an origin, such as `https://destack.app/oauth`. */
    static issuer(origin: string): string {
        return `${new URL(origin).origin}${OAUTH_PATH}`;
    }

    /** Read what a path addresses below a universe's origin: its issuer, a link or a package. */
    static path(pathname: string): "issuer" | "link" | "package" | undefined {
        // read the issuer's path
        if (pathname === OAUTH_PATH || pathname.startsWith(`${OAUTH_PATH}/`)) {
            return "issuer";
        } else if (UniverseOrigin.link(pathname) !== undefined) {
            return "link";
        } else if (PackageMount.parse(pathname) !== undefined) {
            return "package";
        }

        return undefined;
    }

    /** Read the app or repository link a path holds below `/@<handle>`. */
    static link(pathname: string): UniverseLink | undefined {
        // split the path below its `@`
        const [handle, space, name, ...rest] = pathname.replace(/\/$/u, "").split("/").slice(1);
        if (handle === undefined || !handle.startsWith("@")) {
            return undefined;
        }
        const account = handle.slice(1);
        const isHandle = AccountHandle.safeParse(account).success;
        const repository = RepositoryName.safeParse(space?.replace(/\.git$/u, ""));
        const below = [name, ...rest].filter((each) => each !== undefined);

        // read a repository with the Git path below it
        if (isHandle && space?.endsWith(".git") === true && repository.success) {
            const path = below.length === 0 ? "" : `/${below.join("/")}`;

            return { kind: "repository", handle: account, name: repository.data, path };
        }
        // read an app
        else if (
            isHandle &&
            space !== undefined &&
            name !== undefined &&
            rest.length === 0 &&
            SpaceName.safeParse(space).success &&
            InstallationName.safeParse(name).success
        ) {
            return { kind: "app", installation: { name, space, handle: account } };
        }

        return undefined;
    }

    /** The URL of the universe's issuer, such as `https://destack.app/oauth`. */
    get issuer(): string {
        return UniverseOrigin.issuer(this.origin);
    }

    /** Build the URL of one of the universe's pages, such as `https://destack.app/sign-in`. */
    page(page: keyof typeof UNIVERSE_PAGES): URL {
        return new URL(UNIVERSE_PAGES[page], this.origin);
    }

    /** The universe's own domain, its origin's host name. */
    get domain(): string {
        return new URL(this.origin).hostname;
    }

    /** Read what a URL addresses at the universe's origin: its issuer, a link or a package. */
    serves(url: string): "issuer" | "link" | "package" | undefined {
        const { origin, pathname } = new URL(url);

        return origin === this.origin ? UniverseOrigin.path(pathname) : undefined;
    }

    /** Report whether a hostname or wildcard lies at or below one of the universe's domains. */
    covers(hostname: string): boolean {
        return [this.domain, ...Object.values(this.domains)].some(
            (domain) => hostname === domain || hostname.endsWith(`.${domain}`),
        );
    }

    /** Write the URL a name below the universe's domains answers at: its scheme and port. */
    url(hostname: string): string {
        const url = new URL(this.origin);
        url.hostname = hostname;

        return url.origin;
    }

    /** Write the URL a machine's address answers at below the machines' domain. */
    machine(address: MachineAddress): string {
        return this.url(MachineAddress.format(address, this.domains.machine));
    }

    /** Write the URL an installation origin answers at, a loopback machine's below its own port. */
    installation(origin: InstallationOrigin, machine: string): string {
        // keep a loopback machine's port below the loopback domain
        const url = new URL(machine);
        if (url.hostname === LOOPBACK_DOMAIN || HostAddress.isLoopback(url.hostname)) {
            url.hostname = InstallationOrigin.format(origin, LOOPBACK_DOMAIN);

            return url.origin;
        }

        return this.url(InstallationOrigin.format(origin, this.domains.space));
    }

    /** Write the universe's address as JSON. */
    toJSON(): schema.Infer<typeof UniverseOrigin.schema> {
        return { origin: this.origin, domains: this.domains };
    }
}

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

        // read a label without an installation's name as its space's Home
        if (before === undefined && space === undefined) {
            return readFront(names);
        }
        // refuse a branch's label without a space
        else if (space === undefined) {
            return undefined;
        }
        // read the name of the space itself
        else if (before === undefined) {
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
        // write the label of the installation, a branch or a branch digest
        const label =
            origin.digest !== undefined
                ? branchedLabel(origin.digest, origin)
                : origin.branch === undefined
                  ? installationLabel(origin)
                  : branchedLabel(`${origin.name}${NAME_SEPARATOR}${origin.branch}`, origin);

        return domain === undefined ? label : `${label}.${domain}`;
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

/** Write an installation's label, leaving out the defaults: Home, then the home space. */
function installationLabel(origin: {
    readonly name: string;
    readonly space: string;
    readonly handle: string;
}): string {
    // leave out the home installation's name, then the home space's
    if (origin.name !== HOME_APPLICATION) {
        return [origin.name, origin.space, origin.handle].join(NAME_SEPARATOR);
    } else if (origin.space !== HOME_SPACE) {
        return [origin.space, origin.handle].join(NAME_SEPARATOR);
    } else {
        return origin.handle;
    }
}

/** Write a branch's label. */
function branchedLabel(before: string, origin: SpaceOrigin): string {
    // use the readable label while it fits
    const after = `${BRANCH_SEPARATOR}${origin.space}${NAME_SEPARATOR}${origin.handle}`;
    const readable = `${before}${after}`;

    return readable.length <= LABEL_LENGTH ? readable : `${labelDigest(before)}${after}`;
}

/** Read a space's Home from a label's names: a space and handle, or a handle for its home space. */
function readFront(names: readonly string[]): InstallationOrigin | undefined {
    // read a space and handle, else a handle alone
    const [first, second, ...rest] = names;
    const space = readSpace(names);
    const isHandle =
        first !== undefined && second === undefined && AccountHandle.safeParse(first).success;
    if (rest.length > 0) {
        return undefined;
    } else if (space !== undefined) {
        return { name: HOME_APPLICATION, ...space };
    } else if (isHandle) {
        return { name: HOME_APPLICATION, space: HOME_SPACE, handle: first };
    } else {
        return undefined;
    }
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
