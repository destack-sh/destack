import { defineService } from "@destack/service";
import { account, user, zone } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import * as object from "../object/index.ts";
import { relay } from "./relay.ts";

/** The object types space administration serves. */
export const spaceObjects = {
    space: object.space,
    resource: object.resource,
    installation: object.installation,
    installationRevision: object.installationRevision,
    binding: object.binding,
    networkPolicy: object.networkPolicy,
    networkPolicyVersion: object.networkPolicyVersion,
    packagePolicy: object.packagePolicy,
    packagePolicyVersion: object.packagePolicyVersion,
    deployment: object.deployment,
    capture: object.capture,
    instance: object.instance,
    transfer: object.transfer,
    resourceTransfer: object.resourceTransfer,
    snapshot: object.snapshot,
    restoration: object.restoration,
    run: object.run,
    schedule: object.schedule,
    setting,
    user,
    account,
    zone,
} as const;

/** HTTP procedures exposed by space administration. */
export const spaceService = defineService("space", {
    /** Spaces, their installations, and the records their controllers write. */
    objects: spaceObjects,
    /** The relay of the space's zone to the target of its transfer. */
    relay,
});
