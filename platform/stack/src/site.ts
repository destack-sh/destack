/// <reference path="../.sst/platform/config.d.ts" />

import { siteDomain } from "./domains";
import { redirects } from "./redirects";

export function site(stage: string) {
    const website = new sst.aws.StaticSite("Website", {
        path: "../site",
        build: {
            command: "bun run build",
            output: ".output/public",
        },
        assets: {
            fileOptions: [
                {
                    files: "**",
                    cacheControl: "max-age=31536000,public,immutable",
                },
                {
                    files: "index.html",
                    cacheControl: "max-age=0,no-cache,no-store,must-revalidate",
                },
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
    });

    redirects(stage);

    return {
        website: website.url,
    };
}
