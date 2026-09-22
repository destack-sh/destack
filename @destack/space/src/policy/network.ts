import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";

/** A destination selected independently of its protocol and port. */
export const NetworkDestination = defineSchema(
    schema.union([
        schema.object({
            /** Globally routable destinations, excluding host and provider control endpoints. */
            kind: schema.literal("public"),
        }),
        schema.object({
            /** Match a canonical DNS hostname. */
            kind: schema.literal("hostname"),
            /** Lowercase ASCII hostname, without a trailing dot or wildcard. */
            hostname: schema
                .string()
                .max(253)
                .regex(
                    /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/,
                ),
            /** Include descendants of this hostname. */
            subdomains: schema.boolean(),
        }),
        schema.object({
            /** Match the actual destination address against an IP network. */
            kind: schema.literal("network"),
            /** An IPv4 or IPv6 address range; use /32 or /128 for a single address. */
            cidr: schema.union([schema.cidrv4(), schema.cidrv6()]),
        }),
    ]),
);
/** A network destination selector. */
export type NetworkDestination = schema.Infer<typeof NetworkDestination>;

/** An outbound connection rule. */
export const NetworkRule = defineSchema(
    schema.object({
        /** The destination to match, checked again after redirects and DNS resolution. */
        destination: NetworkDestination,
        /** The application protocol or raw transport. */
        protocol: schema.enum(["http", "https", "ws", "wss", "tcp", "udp"]),
        /** Matching ports; omission uses protocol defaults, or any raw transport port. */
        ports: schema.array(schema.int().min(1).max(65535)).min(1).optional(),
        /** Deny takes precedence over allow within this policy. */
        decision: schema.enum(["allow", "deny"]),
    }),
);
/** A named network rule. */
export type NetworkRule = schema.Infer<typeof NetworkRule>;

/**
 * Outbound network restrictions for an account, space, installation, or workload.
 */
export const NetworkPolicyDefinition = defineSchema(
    schema.object({
        /** The decision for unmatched public destinations. */
        default: schema.enum(["allow", "deny"]),
        /** Named rules; nonpublic destinations require an explicit IP network allow rule. */
        rules: schema.record(ResourceName, NetworkRule),
    }),
);
/** A declared outbound network policy. */
export type NetworkPolicyDefinition = schema.Infer<typeof NetworkPolicyDefinition>;
