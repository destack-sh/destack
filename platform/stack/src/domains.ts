/// <reference path="../.sst/platform/config.d.ts" />

import { isProductionStage } from "./stage";

export const canonicalDomain = "destack.sh";

export const redirectDomains = [
    "destack.app",
    "destack.blog",
    "destack.cloud",
    "destack.computer",
    "destack.design",
    "destack.dev",
    "destack.me",
    "destack.site",
    "destack.software",
    "destack.studio",
    "destack.tech",
];

export const redirectHosts = [
    "www.destack.sh",
    ...redirectDomains.flatMap((domain) => [domain, `www.${domain}`]),
];

export function zoneDomain(host: string) {
    if (host.startsWith("www.")) {
        return host.slice("www.".length);
    }

    return host;
}

export function siteDomain(stage: string) {
    if (!isProductionStage(stage)) {
        return undefined;
    }

    return {
        name: canonicalDomain,
        dns: sst.cloudflare.dns(),
    };
}
