import { principal } from "@destack/access";
import { residency, space } from "@destack/account/object";
import { Issuer } from "@destack/connection/object";
import type { MethodContextOf } from "@destack/object";
import { present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { ForgeError } from "../error/index.ts";
import type { GitHubApp } from "../github/index.ts";
import { type AppInstallation, appInstallation, issuer } from "../object/index.ts";

/** Implement the GitHub App's issuers over its key, issuing for claimed installations alone. */
export function implementIssuer(app: GitHubApp | undefined): typeof issuer {
    return issuer.implement({
        methods: {
            create: {
                // relate the residency whose home declares the issuer
                creation: async (context) => {
                    const homed = await context
                        .read(residency)
                        .findFirst({ where: { space: space.identifier(context.scope) } });
                    const code = present(homed, `the residency of ${context.scope}`).id;

                    return {
                        relationships: [
                            {
                                relation: "residency",
                                subject: residency.reference(Scope.universe.id, code),
                            },
                        ],
                    };
                },
                // name the issuer its home's stack declares by the declaration's name
                handle: async (context, next) => {
                    // derive the identifier from the home and the declaration's name
                    const manager = present(context.manager, "the issuer's declaration");
                    const name = manager.name.slice(manager.name.lastIndexOf("/") + 1);
                    const id = await Issuer.declared(
                        { id: context.scope },
                        name,
                        context.environment.crypto,
                    );

                    return next(context.with({ id }));
                },
            },
            issue: {
                prepare: async (context) => {
                    await requireClaimed(context, context.input.installation);

                    return requireApp(app).issue(
                        context.input.installation,
                        context.input.grant ?? {},
                        context.now,
                    );
                },
                handle: async (context) => context.prepared,
            },
            uninstall: {
                prepare: async (context) => {
                    await requireClaimed(context, context.input.installation);
                    await requireApp(app).uninstall(context.input.installation, context.now);

                    return {};
                },
                handle: async (context) => context.prepared,
            },
            installation: async (context) => {
                // read the installation the redirect names, refusing one another installation claimed
                const installation = requireApp(app).installation(context.input.parameters);
                const claimer = requireInstallation(context);
                const claimed = await claimOf(context, installation);
                if (claimed !== undefined && claimed.claimer !== claimer.id) {
                    throw ForgeError.of(
                        "INSTALLATION_CLAIMED",
                        `GitHub installation ${installation} is claimed by another installation`,
                    );
                }

                // claim it for the calling installation and its account once
                if (claimed === undefined) {
                    const row = await context
                        .read(space)
                        .findFirst({ where: { id: space.identifier(claimer.scope) } });
                    const accountId = present(row, `the space ${claimer.scope}`).scope;
                    await context.mutate(context.self, [
                        appInstallation.call("create", {
                            ...appInstallation.in(context.scope),
                            installation,
                            accountId,
                            claimer: claimer.id,
                        }),
                    ]);
                }

                return { installation };
            },
        },
    });
}

/** Require the GitHub App the forge's setting selects. */
function requireApp(app: GitHubApp | undefined): GitHubApp {
    if (app === undefined) {
        throw ForgeError.of("GITHUB_APP_MISSING", "the forge runs no GitHub App");
    }

    return app;
}

/** Require an installation calling the issuer, as connections do. */
function requireInstallation(
    context: Pick<MethodContextOf<typeof issuer, "installation">, "caller">,
): { readonly id: string; readonly scope: string } {
    const { subject } = context.caller;
    if (subject === undefined || !principal.installation.is(subject)) {
        throw ForgeError.of(
            "INSTALLATION_NOT_CLAIMED",
            "only a connection installation calls the GitHub App's issuer",
        );
    }

    return subject;
}

/** Read the claim of a GitHub installation in the issuer's home, absent for an unclaimed one. */
async function claimOf(
    context: Pick<MethodContextOf<typeof issuer, "installation">, "read" | "scope">,
    installation: string,
): Promise<AppInstallation | undefined> {
    return context
        .read(appInstallation)
        .findFirst({ where: { scope: space.identifier(context.scope), installation } });
}

/** Require the calling installation to have claimed a GitHub installation. */
async function requireClaimed(
    context: Pick<MethodContextOf<typeof issuer, "issue">, "read" | "scope" | "caller">,
    installation: string,
): Promise<void> {
    const claimed = await claimOf(context, installation);
    if (claimed?.claimer !== requireInstallation(context).id) {
        throw ForgeError.of(
            "INSTALLATION_NOT_CLAIMED",
            `the calling installation has not claimed GitHub installation ${installation}`,
        );
    }
}
