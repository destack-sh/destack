import { ResidencyCode } from "@destack/account/object";
import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";

/** The origin a relay installation answers at, where machines open their tunnels. */
export const origin = defineSetting({
    name: "origin",
    title: t`Relay origin`,
    description: t`The origin machines open their tunnels at and names reach the relay at.`,
    schema: schema.url(),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

/** The residency a relay installation serves, whose accounts' machines alone tunnel to it. */
export const residency = defineSetting({
    name: "residency",
    title: t`Residency`,
    description: t`The residency whose accounts' machines open their tunnels at this relay.`,
    schema: ResidencyCode,
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

/** The issuer of the universe's tokens machines open their tunnels with. */
export const issuer = defineSetting({
    name: "issuer",
    title: t`Universe issuer`,
    description: t`The issuer of the universe's tokens machines open their tunnels with.`,
    schema: schema.url(),
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});
