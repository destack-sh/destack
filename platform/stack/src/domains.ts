/// <reference path="../.sst/platform/config.d.ts" />

import { isProductionStage } from "./stage";

export const canonicalDomain = "destack.sh";

const redirectDomains = [
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

export function siteDomain(stage: string) {
    if (!isProductionStage(stage)) {
        return undefined;
    }

    return {
        name: canonicalDomain,
        dns: sst.cloudflare.dns(),
        redirects: ["www.destack.sh", ...redirectDomains.flatMap((domain) => [domain, `www.${domain}`])],
    };
}
