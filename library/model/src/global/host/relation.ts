import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { hostAccess } from "./access.ts";
import { deviceKey } from "./key.ts";
import { device } from "./device.ts";
import { host } from "./host.ts";
import { region } from "./region.ts";
import { tunnel } from "./tunnel.ts";

/** Query relationships for host records. */
export const hostRelations = defineRelationsPart(
    {
        host,
        hostAccess,
        account,
        device,
        deviceKey,
        region,
        tunnel,
    },
    (relation) => ({
        hostAccess: {
            host: relation.one.host({
                from: [relation.hostAccess.hostId],
                to: [relation.host.id],
                optional: false,
            }),
            account: relation.one.account({
                from: [relation.hostAccess.accountId],
                to: [relation.account.id],
                optional: false,
            }),
        },
        deviceKey: {
            device: relation.one.device({
                from: [relation.deviceKey.deviceId],
                to: [relation.device.id],
                optional: false,
            }),
        },
        device: {
            account: relation.one.account({
                from: [relation.device.accountId],
                to: [relation.account.id],
                optional: false,
            }),
        },
        host: {
            device: relation.one.device({
                from: [relation.host.accountId, relation.host.deviceId],
                to: [relation.device.accountId, relation.device.id],
                optional: true,
            }),
            account: relation.one.account({
                from: [relation.host.accountId],
                to: [relation.account.id],
                optional: false,
            }),
            region: relation.one.region({
                from: [relation.host.regionId],
                to: [relation.region.id],
                optional: true,
            }),
        },
        tunnel: {
            host: relation.one.host({
                from: [relation.tunnel.deviceId, relation.tunnel.hostId],
                to: [relation.host.deviceId, relation.host.id],
                optional: false,
            }),
            key: relation.one.deviceKey({
                from: [relation.tunnel.deviceId, relation.tunnel.deviceKeyId],
                to: [relation.deviceKey.deviceId, relation.deviceKey.id],
                optional: false,
            }),
            device: relation.one.device({
                from: [relation.tunnel.deviceId],
                to: [relation.device.id],
                optional: false,
            }),
        },
    }),
);
