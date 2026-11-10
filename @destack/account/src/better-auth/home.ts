import type { BetterAuthPlugin } from "better-auth";
import { createAuthEndpoint, getSessionFromCtx } from "better-auth/api";
import type { Clock } from "@destack/environment";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { present } from "@destack/schema";
import { InstallationOrigin } from "../address/address.ts";
import { Directory } from "../address/directory.ts";
import { HOME_LOCATION_PATH } from "../authentication/path.ts";
import { HomeLocation } from "../authentication/request.ts";
import { AccountError } from "../error/error.ts";
import { refusing } from "./refusal.ts";
import { account, Account, HOME_APPLICATION, HOME_SPACE } from "../object/account.ts";

/** A person's home: where Home answers in their personal account's home space. */
export const Home = {
    /** Read where a person's Home answers, starting while no machine serves their home. */
    async locate(userId: string, database: DatabaseConnection, now: number): Promise<HomeLocation> {
        // find the machine answering for the person's home space
        const accountId = Account.personal(userId);
        const snapshot = Snapshot.live(database);
        const located = await new Directory(snapshot).serving(Account.home(accountId), now);
        if (located === undefined) {
            return { status: "starting" };
        }

        // address Home's installation origin in the home space of the person's account
        const owner = await snapshot.read(account.table).findFirst({ where: { id: accountId } });
        const handle = present(owner, `the account of ${userId}`).handle;
        const origin = { name: HOME_APPLICATION, space: HOME_SPACE, handle };

        return { status: "serving", url: `${InstallationOrigin.url(origin, located.url)}/` };
    },

    /** Serve where the signed-in person's Home answers, for the views at the universe's origin. */
    plugin(database: DatabaseConnection, clock: Clock): BetterAuthPlugin {
        return {
            id: "destack-home",
            endpoints: {
                /** Report where the signed-in person's Home answers. */
                locateHome: createAuthEndpoint(
                    HOME_LOCATION_PATH,
                    { method: "GET" },
                    async (context) =>
                        await refusing(async () => {
                            // refuse a signed-out browser
                            const session = await getSessionFromCtx(context);
                            if (session === null) {
                                throw AccountError.of(
                                    "SIGN_IN_REQUIRED",
                                    "the person is not signed in",
                                );
                            }
                            const located = await Home.locate(
                                session.user.id,
                                database,
                                clock.now(),
                            );

                            return context.json(HomeLocation.parse(located));
                        }),
                ),
            },
        };
    },
};
