import { TEST_DIALECTS } from "@destack/db/test";
import { isServiceError } from "@destack/service/error";
import { expect, onTestFinished, test } from "@destack/test";
import { entitlement, subscriptionItem } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { capacity, storage, stored } from "./fixture/storage.ts";
import { CatalogReference } from "@destack/finance";

test.each(TEST_DIALECTS)(
    "receive a planned account's usage from a workload whose token claims the account's residency, and refuse one claiming none or another residency and one whose token lapsed on %s",
    async (dialect) => {
        // copy the destack account, whose residency's workloads serve it, granted its plan's storage
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        await finance.server.executeAsSystem(
            entitlement,
            "create",
            [
                {
                    scope: ids.destack,
                    input: {
                        packageId: storage.id,
                        feature: capacity.name,
                        kind: "metered",
                        limit: 1000,
                        usage: 0,
                        state: "within",
                        source: subscriptionItem.reference(ids.destack, "subscription-item-1"),
                    },
                },
            ],
            Date.now(),
        );

        // route a use as the cell under each token
        const append = async (claims: Parameters<typeof finance.fixture.workload>[1], id: string) =>
            finance.fixture
                .workloadEvents("cell", claims)
                .receive({
                    kind: "usage",
                    events: [
                        {
                            scope: ids.destack,
                            id,
                            source: "/spaces/a",
                            time: Date.now() * 1000,
                            keys: {
                                meter: CatalogReference.key({
                                    packageId: storage.id,
                                    name: stored.name,
                                }),
                                sku: null,
                                installation: null,
                                quantity: 1,
                                level: 1,
                            },
                            data: { unit: "byte", from: Date.now(), to: Date.now() },
                        },
                    ],
                })
                .then(
                    () => "appended",
                    (error: unknown) => (isServiceError(error) ? error.code : String(error)),
                );

        expect([
            await append({ residencies: ["eu"] }, "1"),
            await append({ residencies: [] }, "2"),
            await append({ residencies: ["us"] }, "3"),
            await append({ residencies: ["eu"], lapsesAt: Date.now() - 1 }, "4"),
        ]).toEqual(["appended", "NOT_FOUND", "NOT_FOUND", "UNAUTHORIZED"]);
    },
);
