import { UniverseOrigin } from "@destack/account/address";
import { RegionCode } from "@destack/account/object";
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

/** The region a relay installation serves, whose accounts' machines alone tunnel to it. */
export const region = defineSetting({
    name: "region",
    title: t`Region`,
    description: t`The region whose accounts' machines open their tunnels at this relay.`,
    schema: RegionCode,
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});

/** The universe whose machines open their tunnels at a relay installation. */
export const universe = defineSetting({
    name: "universe",
    title: t`Universe`,
    description: t`The universe whose machines open their tunnels at this relay.`,
    schema: UniverseOrigin.schema,
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
    required: true,
});
