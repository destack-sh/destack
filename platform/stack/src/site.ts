/// <reference path="../.sst/platform/config.d.ts" />

import { siteDomain } from "./domains";

export function site(stage: string) {
    const site = new sst.aws.StaticSite("Site", {
        path: "../site",
        build: {
            command: "bun run build",
            output: ".output/public",
        },
        assets: {
            fileOptions: [
                {
                    files: ["install", "install.ps1"],
                    cacheControl: "max-age=0,no-cache,no-store,must-revalidate",
                },
                {
                    files: "install",
                    contentType: "text/x-shellscript",
                },
                {
                    files: "install.ps1",
                    contentType: "text/plain",
                },
            ],
        },
        domain: siteDomain(stage),
        edge: {
            viewerRequest: {
                injection: `
if (event.request.uri === "/install") {
    return event.request;
}
`,
            },
        },
    });

    return {
        site: site.url,
    };
}
