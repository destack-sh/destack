import { account } from "./account.ts";
import { check, foreignKey, sql, unique, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";

/** The domains the platform serves itself, which no account claims or nests under. */
export const RESERVED_DOMAINS = [
    "destack.sh",
    "destack.app",
    "destack.space",
    "destack.cloud",
    "destack.computer",
];

/** The DNS label of the TXT record proving a claim, below the claimed hostname. */
const CHALLENGE_LABEL = "_destack";

/** One DNS label: letters, digits and inner hyphens, 1 to 63 characters (RFC 1035 2.3.1). */
const LABEL = "[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?";

/** A lowercase ASCII hostname of two or more labels, with a top-level label that is not numeric. */
const HOSTNAME = new RegExp(`^(?:${LABEL}\\.)+(?=[a-z0-9-]*[a-z])${LABEL}$`);

/** Any platform domain or a hostname below one. */
const RESERVED = new RegExp(
    `^(?!(?:.*\\.)?(?:${RESERVED_DOMAINS.map((reserved) => RegExp.escape(reserved)).join("|")})$)`,
);

/** A DNS hostname in its canonical lowercase ASCII form, at most 253 characters (RFC 1035 2.3.4). */
export const Hostname = schema
    .string()
    .max(253)
    .regex(HOSTNAME, "hostnames are lowercase ASCII labels, punycode for others")
    .regex(RESERVED, "platform domains are reserved");

/** The TXT record proving an account's claim to a hostname. */
export const DomainChallenge = {
    /** Read the record a claim's administrators publish: its name and its value. */
    of(domain: { readonly id: string; readonly hostname: string }): {
        name: string;
        value: string;
    } {
        return {
            name: `${CHALLENGE_LABEL}.${domain.hostname}`,
            value: `destack-verify=${domain.id}`,
        };
    },
};

/** A DNS hostname an account claims, and serves once it proves the claim. */
export const domain = defineObject({
    name: "domain",
    tier: "global",
    plural: "domains",
    scope: account,
    fields: {
        /** The canonical lowercase DNS hostname. */
        hostname: field.string(Hostname),
        /** The time the account proved its claim through the hostname's TXT record. */
        verifiedAt: field.time().optional(),
    },
    permissions: ["read", "create", "verify", "delete"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["hostname"] }),
        verify: method({ permission: "verify" }),
        delete: method.delete("delete"),
    },
    constraints: (entry) => [
        foreignKey({ columns: [entry.scope], foreignColumns: [account.table.id] }).onDelete(
            "restrict",
        ),
        unique("domain_scope_hostname").on(entry.scope, entry.hostname),
        uniqueIndex("domain_verified_hostname")
            .on(entry.hostname)
            .where(sql`${entry.verifiedAt} IS NOT NULL`),
        check("domain_hostname_form", sql`${entry.hostname} = lower(${entry.hostname})`),
    ],
});

/** A persisted domain record. */
export type Domain = Select<typeof domain.table>;
