import { TABLE } from "@destack/db";
import { schema } from "@destack/schema";
import { RequestId, type Page, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { account, residentsShape } from "../src/object/index.ts";
import { AccountFixture } from "../src/test/index.ts";

/** How long a live replica may take to deliver a change: a second on a loaded machine. */
const CHANGE_MILLISECONDS = 1000;

/** List the account identifiers a page of a replica adds. */
function added(page: Page): string[] {
    return page.changes
        .filter((change) => change.table === account.table[TABLE].sqlName && change.row !== null)
        .map((change) => schema.string().parse(change.row["id"]))
        .toSorted();
}

test("replicate every account of a residency to its cloud machine as it is created, and none of another residency", async () => {
    // follow the eu accounts as the cloud machine serving the owner's home
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { machine } = await fixture.personalHome(owner);
    const controller = new AbortController();
    const pages = (
        await machine.client.store.stream(
            residentsShape.subscription({ scope: Scope.universe.id, residency: "eu" }, machine.id),
            { signal: controller.signal },
        )
    )[Symbol.asyncIterator]();
    const first = await pages.next();

    // create an account in the us, then a person's eu account, while the replica runs
    const overseas = await owner.client.account.create({
        kind: "shared",
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        handle: "overseas",
        name: "Overseas",
        residencyId: "us",
    });
    const later = await fixture.person("later@example.com");

    // read the changes the replica delivers next
    const delivered: string[] = [];
    const deadline = Date.now() + CHANGE_MILLISECONDS;
    while (!delivered.includes(later.account) && Date.now() < deadline) {
        const next = await Promise.race([
            pages.next(),
            new Promise<undefined>((resolve) => {
                setTimeout(resolve, deadline - Date.now());
            }),
        ]);
        if (next === undefined || next.done === true) {
            break;
        }
        delivered.push(...added(next.value));
    }
    controller.abort();

    expect({
        isOwnerReplicated: first.done !== true && added(first.value).includes(owner.account),
        isLaterDelivered: delivered.includes(later.account),
        isOverseasDelivered: delivered.includes(overseas.id),
    }).toEqual({ isOwnerReplicated: true, isLaterDelivered: true, isOverseasDelivered: false });
});
