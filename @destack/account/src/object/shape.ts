import { defineShape } from "@destack/object";
import { schema } from "@destack/schema";
import { account } from "./account.ts";
import { claim } from "./claim.ts";
import { endpoint } from "./endpoint.ts";
import { identity, operation } from "./identity.ts";
import { key } from "./key.ts";
import { machine } from "./machine.ts";
import { organization } from "./organization.ts";
import { assignment, placement } from "./placement.ts";
import { residency, ResidencyCode } from "./residency.ts";
import { space } from "./space.ts";
import { user } from "./user.ts";

/** A space's rows as its machine serves it: row, placement, assignments, machine and residency. */
export const spaceShape = defineShape({
    name: "space",
    parameters: schema.object({
        /** The space. */
        space: schema.identifier("space"),
        /** The machine serving it. */
        machine: schema.identifier("machine"),
    }),
    objects: [space, placement, assignment, machine, residency],
    queries: ({ space: id, machine: machineId }) => [
        space.query({ scopes: "every", where: { id } }),
        placement.query({ scopes: "every", where: { id } }),
        assignment.query({ scopes: "every", where: { placementId: id } }),
        machine.query({ scopes: "every", where: { id: machineId } }),
        // the residencies, showing a home its own, which its residents follow keys on
        residency.query({ scopes: "every" }),
    ],
});

/** The rows of one space alone, which a store reads to decide for that space as a caller. */
export const spaceRecordShape = defineShape({
    name: "space-record",
    parameters: schema.object({
        /** The space. */
        space: schema.identifier("space"),
    }),
    objects: [space],
    queries: ({ space: id }) => [space.query({ scopes: "every", where: { id } })],
});

/** One account's rows: its row and its machines. */
export const accountShape = defineShape({
    name: "account",
    parameters: schema.object({ account: schema.identifier("account") }),
    objects: [account, machine],
    queries: ({ account: id }) => [
        account.query({ scopes: "every", where: { id } }),
        machine.query({ scopes: "every", where: { scope: id } }),
    ],
});

/** One organization's row, which has its payer. */
export const organizationShape = defineShape({
    name: "organization",
    parameters: schema.object({ organization: schema.identifier("organization") }),
    objects: [organization],
    queries: ({ organization: id }) => [organization.query({ scopes: "every", where: { id } })],
});

/** One machine's rows beside the directory: the work the spaces assign it. */
export const machineShape = defineShape({
    name: "machine",
    parameters: schema.object({ machine: schema.identifier("machine") }),
    objects: [assignment],
    queries: ({ machine: id }) => [assignment.query({ where: { machineId: id } })],
});

/** The directory machines and routers read: placements, names, machines and endpoints. */
export const directoryShape = defineShape({
    name: "directory",
    parameters: schema.object({}),
    objects: [placement, claim, machine, key, endpoint],
    queries: () => [
        placement.query({ scopes: "every" }),
        claim.query({ scopes: "every" }),
        machine.query({ scopes: "every" }),
        // replicate the keys of machines alone, leaving out people's clients
        key.query({
            scopes: "every",
            where: { parentPackageId: machine.typeReference.packageId, parentType: machine.name },
        }),
        endpoint.query({ scopes: "every" }),
    ],
});

/** The identities and their logs, which the universe and spaces sign with. */
export const identityShape = defineShape({
    name: "identity",
    parameters: schema.object({}),
    objects: [identity, operation],
    queries: () => [identity.query({ scopes: "every" }), operation.query({ scopes: "every" })],
});

/** One claimed name, by its index and key, which a store follows on demand. */
export const claimShape = defineShape({
    name: "claim",
    parameters: schema.object({ index: schema.string().min(1), key: schema.string().min(1) }),
    objects: [claim],
    queries: (claimed) => [
        claim.query({ scopes: "every", where: { index: claimed.index, key: claimed.key } }),
    ],
});

/** Every account of a residency, which its home space serves. */
export const residentsShape = defineShape({
    name: "residents",
    parameters: schema.object({
        /** The residency whose accounts are replicated. */
        residency: ResidencyCode,
    }),
    objects: [account],
    queries: ({ residency: residencyId }) => [
        account.query({ scopes: "every", where: { residencyId } }),
    ],
});

/** One person's public profile, which a space follows for each person related to it. */
export const profileShape = defineShape({
    name: "profile",
    parameters: schema.object({ user: schema.identifier("user") }),
    objects: [user],
    queries: ({ user: id }) => [user.query({ scopes: "every", where: { id } })],
});

/** The spaces of an account. */
export const spacesShape = defineShape({
    name: "spaces",
    parameters: schema.object({ account: schema.identifier("account") }),
    objects: [space],
    queries: ({ account: id }) => [space.query({ scopes: "every", where: { scope: id } })],
});
