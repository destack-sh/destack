import { through, union } from "@destack/access";
import { type Select } from "@destack/db";
import { account } from "./account.ts";
import { organization } from "./organization.ts";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The DNS label of the TXT record proving a claim, below the claimed hostname. */
const CHALLENGE_LABEL = "_destack";

/** The longest hostname in its text form (RFC 1035 2.3.4). */
const HOSTNAME_LENGTH = 253;

/** One DNS label: letters, digits and hyphens between them, 1 to 63 characters (RFC 1035 2.3.1). */
const LABEL = "[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?";

/** A lowercase ASCII hostname of two or more labels. */
const HOSTNAME = new RegExp(`^(?:${LABEL}\\.)+${LABEL}$`, "u");

/** A top-level label with a letter, which tells a hostname from an address. */
const TOP_LEVEL = /[a-z][a-z0-9-]*$/u;

/** A hostname, or a wildcard label before one matching every name below it. */
const HOSTNAME_PATTERN = new RegExp(`^(?:\\*\\.)?${HOSTNAME.source.slice(1)}`, "u");

/** Who administers the domains of an account or an organization. */
const MANAGERS = union(through("account", "manage"), through("organization", "manage"));

/** A DNS hostname in its canonical lowercase ASCII form. */
export const Hostname = schema
    .string()
    .max(HOSTNAME_LENGTH)
    .regex(HOSTNAME, "hostnames are lowercase ASCII labels, punycode for others")
    .regex(TOP_LEVEL, "top-level labels have a letter");

/** A hostname an account serves on, or a wildcard `*.` before one for every name below it. */
export const HostnamePattern = schema
    .string()
    .max(HOSTNAME_LENGTH)
    .regex(HOSTNAME_PATTERN, "hostnames are lowercase ASCII labels, an optional *. first")
    .regex(TOP_LEVEL, "top-level labels have a letter");

/** The TXT record proving the claim to a domain. */
export const DomainChallenge = Object.assign(
    schema.object({
        /** The record's name, below the claimed domain. */
        name: schema.string().min(1),
        /** The record's value, with the claim. */
        value: schema.string().min(1),
    }),
    {
        /** Read the record a claim's administrators publish. */
        of(domain: { readonly scope: string; readonly name: string }): DomainChallenge {
            return {
                name: `${CHALLENGE_LABEL}.${domain.name}`,
                value: `destack-verify=${domain.scope}`,
            };
        },
    },
);
/** The TXT record a domain claim's administrators publish. */
export type DomainChallenge = schema.Infer<typeof DomainChallenge>;

/** A DNS domain an account or an organization proves it controls once, through a TXT record. */
export const domain = defineObject(
    {
        name: "domain",
        plural: "domains",
        scope: [account, organization],
        fields: {
            /** The canonical lowercase domain, such as acme.com. */
            name: field.string(Hostname),
            /** The time the owner proved its claim through the domain's TXT record. */
            verifiedAt: field.time().optional(),
        },
        permissions: {
            read: MANAGERS,
            create: MANAGERS,
            verify: MANAGERS,
            delete: MANAGERS,
        },
        constraints: {
            scopeName: { kind: "unique", on: ["scope", "name"] },
            verifiedName: {
                kind: "index",
                on: ["name"],
                unique: true,
                where: { verifiedAt: { isNotNull: true } },
            },
            nameForm: { kind: "check", condition: { name: { characters: "a-z0-9.*-" } } },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["name"] }),
        verify: method.mutation({ permission: "verify", prepared: DomainChallenge }),
        delete: method.delete("delete"),
    }),
);
/** A persisted domain record. */
export type Domain = Select<typeof domain.table>;

/** The hostnames a domain covers. */
export const Domain = {
    /** Decide whether a domain covers a hostname: the domain itself or a name below it. */
    covers(name: string, hostname: string): boolean {
        return hostname === name || hostname.endsWith(`.${name}`);
    },
};
