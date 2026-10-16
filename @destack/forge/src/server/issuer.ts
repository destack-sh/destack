import type { GitHubApp } from "../github/index.ts";
import { issuer } from "../object/index.ts";

/** Implement the GitHub App's issuers over the app's key. */
export function implementIssuer(app: GitHubApp): typeof issuer {
    return issuer.implement({
        methods: {
            issue: {
                prepare: (context) => app.issue(context.input.installation, context.input.grant),
                handle: async (context) => context.prepared,
            },
            uninstall: {
                prepare: async (context) => {
                    await app.uninstall(context.input.installation);

                    return {};
                },
                handle: async (context) => context.prepared,
            },
            installation: async (call) => ({
                installation: app.installation(call.input.parameters),
            }),
        },
    });
}
