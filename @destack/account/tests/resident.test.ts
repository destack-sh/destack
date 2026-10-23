import accountPackage from "../src/package.ts";
import { TABLE } from "@destack/db";
import { schema } from "@destack/schema";
import { type Page, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Account, account, residentsShape } from "../src/object/index.ts";
import { ids, TestAccountStore, OWNER_EMAIL, PEOPLE } from "../src/test/index.ts";
import { firstPage } from "./fixture.ts";

/** How long the test follows the replica at most, a test's timeout. */
const REPLICA_MILLISECONDS = 30_000;

/** List the account identifiers a page of a replica adds. */
function added(page: Page): string[] {
    return page.changes
        .filter((change) => change.table === account.table[TABLE].sqlName && change.row !== null)
        .map((change) => schema.string().parse(change.row["id"]))
        .toSorted();
}

test("replicate every account of a residency to its cloud machine as it is created, and none of another residency", async () => {
    // follow the eu accounts as the cloud machine serving the owner's home, for as long as the replica waits
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { machine } = await fixture.personalHome(owner);
    const controller = new AbortController();
    const signal = AbortSignal.any([
        controller.signal,
        fixture.clock.timeout(REPLICA_MILLISECONDS),
    ]);
    const stream = await machine
        .client(accountPackage)
        .store.stream(
            residentsShape.subscription(
                { scope: Scope.universe.id, residency: "eu" },
                machine.subject.id,
            ),
            { signal },
        );
    const first = await firstPage(stream);
    const pages = stream[Symbol.asyncIterator]();

    // create an account in the us, then a person's eu account, while the replica runs
    await fixture.createAccount(owner, "us");
    const later = await fixture.signIn("later@example.com");

    // read the changes the replica delivers until the eu account arrives
    const delivered: string[] = [];
    for (let next = await pages.next(); next.done !== true; next = await pages.next()) {
        delivered.push(...added(next.value));

        // stop at the eu account
        if (delivered.includes(later.account)) {
            break;
        }
    }
    controller.abort();

    // replicate the residency's accounts first, then the new eu account alone, never the us one
    expect({ first: added(first), delivered }).toEqual({
        first: [
            ids.other,
            ...PEOPLE.map((person) => Account.personal(person.id)),
            fixture.platform,
        ].toSorted(),
        delivered: [later.account],
    });
});
