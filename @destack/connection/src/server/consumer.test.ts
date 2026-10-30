import { principal } from "@destack/access";
import { RequestId } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { connection } from "../object/index.ts";
import { TestConnection } from "../test/index.ts";

/** Serve connections for one test. */
async function open(): Promise<TestConnection> {
    const fixture = await TestConnection.open();
    onTestFinished(() => fixture.close());

    return fixture;
}

/** Create an authorized key connection as the owner, answering its identifier. */
async function createKey(fixture: TestConnection, input: Readonly<Record<string, unknown>> = {}) {
    const created = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(fixture.environment),
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

test("lease a connection to an installation's calls only while it is bound to that installation", async () => {
    const fixture = await open();
    const id = await createKey(fixture);
    const reference = connection.reference(fixture.scope.id, id);
    const bridge = principal.installation.reference(fixture.scope.id, "installation-bridge");
    const other = principal.installation.reference(fixture.scope.id, "installation-other");
    const lease = (installation: typeof bridge) =>
        refusal(
            fixture
                .lease(reference, new Request("https://api.test/items"), installation)
                .then((leased) => leased.request.headers.get("x-api-key")),
        );
    const change = (name: "bindConsumer" | "releaseConsumer") =>
        fixture.objects.mutate(fixture.context().access(), {
            id: RequestId.create(fixture.environment),
            calls: [
                connection.call(name, {
                    ...connection.target({ scope: fixture.scope.id, id }),
                    consumer: bridge,
                }),
            ],
        });

    // refuse before leasing, lease to the bridge alone, and refuse again once reclaimed, hiding the scope from installations not bound
    const before = await lease(bridge);
    await change("bindConsumer");
    const leased = [await lease(bridge), await lease(other)];
    await change("releaseConsumer");
    const hidden = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${fixture.scope.id} not found`];

    expect({ before, leased, reclaimed: await lease(bridge) }).toEqual({
        before: hidden,
        leased: ["done", hidden],
        reclaimed: hidden,
    });
});
