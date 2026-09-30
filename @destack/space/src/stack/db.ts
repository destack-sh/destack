import type { Table } from "@destack/db";
import { account, user, zone } from "@destack/account/object";
import { setting } from "@destack/setting/object";
import { binding, capture } from "../object/binding.ts";
import { deployment } from "../object/deployment.ts";
import { installation, installationRevision } from "../object/installation.ts";
import { instance } from "../object/instance.ts";
import {
    networkPolicy,
    networkPolicyVersion,
    packagePolicy,
    packagePolicyVersion,
} from "../object/policy.ts";
import { resource } from "../object/resource.ts";
import { run } from "../object/run.ts";
import { schedule } from "../object/schedule.ts";
import { restoration, snapshot } from "../object/snapshot.ts";
import { space } from "../object/space.ts";
import { resourceTransfer, transfer } from "../object/transfer.ts";
import { spaceJournal } from "./journal.ts";

/** Spaces, installations, resources, deployments, their setting values and access. */
export const spaceTables: readonly Table[] = [
    spaceJournal,
    ...[
        networkPolicy,
        networkPolicyVersion,
        packagePolicy,
        packagePolicyVersion,
        binding,
        capture,
        resource,
        restoration,
        snapshot,
        deployment,
        installation,
        installationRevision,
        instance,
        run,
        schedule,
        space,
        transfer,
        resourceTransfer,
        setting,
        user,
        account,
        zone,
    ].flatMap((object) => object.tables),
];
