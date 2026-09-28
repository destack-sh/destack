import type { NetworkPolicyDefinition, PackagePolicyDefinition } from "@destack/space";

/** Admit Destack packages and packages from the public npm registry. */
export const packages = {
    admission: {
        default: "deny",
        rules: {
            destack: { package: { kind: "destack" }, decision: "allow" },
            npm: {
                package: { kind: "npm", registry: "https://registry.npmjs.org/" },
                decision: "allow",
            },
        },
    },
} satisfies PackagePolicyDefinition;

/** Permit public HTTPS and secure WebSockets; require explicit grants for other destinations. */
export const network = {
    default: "deny",
    rules: {
        https: { destination: { kind: "public" }, protocol: "https", decision: "allow" },
        websocket: { destination: { kind: "public" }, protocol: "wss", decision: "allow" },
    },
} satisfies NetworkPolicyDefinition;
