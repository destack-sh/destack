/// <reference path="../.sst/platform/config.d.ts" />

import * as cloudflare from "@pulumi/cloudflare";

import { canonicalDomain, redirectHosts, zoneDomain } from "./domains";
import { isProductionStage } from "./stage";

export function redirects(stage: string) {
    if (!isProductionStage(stage)) {
        return;
    }

    const accountId = cloudflareAccountId();
    const zones = redirectZones();

    for (const [zoneName, hosts] of zones) {
        const zone = cloudflare.getZoneOutput({
            filter: {
                account: { id: accountId },
                name: zoneName,
            },
        });

        for (const host of hosts) {
            new cloudflare.DnsRecord(`Redirect${resourceKey(host)}Record`, {
                zoneId: zone.zoneId,
                name: host,
                type: "AAAA",
                content: "100::",
                proxied: true,
                ttl: 1,
            });
        }

        new cloudflare.Ruleset(`Redirect${resourceKey(zoneName)}Ruleset`, {
            zoneId: zone.zoneId,
            name: "destack redirects",
            kind: "zone",
            phase: "http_request_dynamic_redirect",
            rules: hosts.map((host) => ({
                action: "redirect",
                description: `redirect ${host} to ${canonicalDomain}`,
                expression: `http.host eq "${host}"`,
                ref: `redirect_${host.replaceAll(".", "_")}`,
                actionParameters: {
                    fromValue: {
                        statusCode: 301,
                        preserveQueryString: true,
                        targetUrl: {
                            expression: `concat("https://${canonicalDomain}", http.request.uri.path)`,
                        },
                    },
                },
            })),
        });
    }
}

function redirectZones() {
    const zones = new Map<string, string[]>();

    for (const host of redirectHosts) {
        const zone = zoneDomain(host);
        const hosts = zones.get(zone) ?? [];

        hosts.push(host);
        zones.set(zone, hosts);
    }

    return zones;
}

function cloudflareAccountId() {
    const accountId =
        process.env.CLOUDFLARE_DEFAULT_ACCOUNT_ID ?? process.env.CLOUDFLARE_ACCOUNT_ID;

    if (!accountId) {
        throw new Error("CLOUDFLARE_ACCOUNT_ID is required");
    }

    return accountId;
}

function resourceKey(host: string) {
    return host
        .split(".")
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join("");
}
