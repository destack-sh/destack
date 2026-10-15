import { principal } from "@destack/access";
import { RequestId } from "@destack/runtime/request";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { connection } from "../object/index.ts";
import { ConnectionFixture } from "../test/index.ts";

/** Serve connections for one test. */
async function open(): Promise<ConnectionFixture> {
    const fixture = await ConnectionFixture.open();
    onTestFinished(() => fixture.close());

    return fixture;
}

/** Create an authorized key connection as the owner, answering its identifier. */
async function createKey(
    fixture: ConnectionFixture,
    input: Readonly<Record<string, unknown>> = {},
) {
    const created = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("create", {
                    scope: fixture.scope.id,
                    vault: fixture.vaultReference,
                    protocol: { kind: "key", header: "x-api-key" },
                    material: { kind: "key", key: "sk-test-0123456789" },
                    ...input,
                }),
            ],
        })
        .then(([written]) => written);
    await fixture.control();

    return created.id;
}

test("lend a connection to an installation's calls only while it is bound to that installation", async () => {
    const fixture = await open();
    const id = await createKey(fixture);
    const reference = connection.reference(fixture.scope.id, id);
    const borrower = principal.installation.reference(fixture.scope.id, "installation-bridge");
    const other = principal.installation.reference(fixture.scope.id, "installation-other");
    const lend = (installation: typeof borrower) =>
        refusal(
            fixture
                .borrow(reference, new Request("https://api.test/items"), installation)
                .then((loan) => loan.request.headers.get("x-api-key")),
        );
    const change = (name: "bindConsumer" | "releaseConsumer") =>
        fixture.objects.mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call(name, {
                    ...connection.target({ scope: fixture.scope.id, id }),
                    consumer: borrower,
                }),
            ],
        });

    // refuse before lending, lend to the borrower alone, and refuse again once reclaimed, hiding the scope from installations not bound
    const before = await lend(borrower);
    await change("bindConsumer");
    const loan = [await lend(borrower), await lend(other)];
    await change("releaseConsumer");
    const hidden = ["NOT_FOUND", `no scope ${fixture.scope.id}`];

    expect({ before, loan, reclaimed: await lend(borrower) }).toEqual({
        before: hidden,
        loan: ["done", hidden],
        reclaimed: hidden,
    });
});
