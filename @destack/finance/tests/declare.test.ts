import { expect, refusal, test } from "@destack/test";
import { defineFeature } from "../src/declare/index.ts";
import { compareFeature, describeFeature, featureSymbols } from "../src/inspect/index.ts";
import { schema } from "@destack/schema";
import { testSimulation } from "@destack/object/test";
import { ReleaseReader } from "../src/release/reader.ts";
import { type Feature, type FeatureOptions } from "../src/feature/index.ts";
import { TestFinance } from "../src/test/index.ts";
import {
    calls,
    storageLimit,
    requests,
    seats,
    storage,
    stored,
    support,
    sync,
} from "./fixture/storage.ts";
import { t } from "@destack/locale";

test("read features and meters back from a release", async () => {
    // read the storage package's declarations back from its build
    const build = await TestFinance.release(
        storage,
        [requests, stored, sync, support, seats, calls, storageLimit],
        testSimulation().environment("finance").crypto,
    );
    const releases = new ReleaseReader(async () => ReleaseReader.read(build));
    const features = [sync, support, seats, calls, storageLimit];
    const described = await Promise.all(
        features.map(async (feature) => describeFeature(await releases.feature(feature.reference))),
    );

    // describe each as declared, and refuse an undeclared feature
    expect([
        described,
        (await releases.meter(requests.reference)).definition,
        await refusal(releases.feature({ packageId: storage.id, name: "backup" })),
    ]).toEqual([
        features.map((feature) => describeFeature(feature)),
        requests.definition,
        ["NOT_FOUND", "FEATURE_NOT_FOUND", `feature '${storage.id}/backup' not found`],
    ]);
});

test("relate a metered feature to its meters", () => {
    expect([featureSymbols(calls), featureSymbols(sync)]).toEqual([
        [
            {
                relationships: [
                    {
                        kind: "reads",
                        symbol: { kind: "meter", name: "api.requests", packageId: storage.id },
                    },
                ],
            },
        ],
        [{ relationships: [] }],
    ]);
});

/** Read a feature's release entry back from JSON. */
function entry(feature: Feature, version: string) {
    return {
        description: schema
            .jsonObject()
            .parse(JSON.parse(JSON.stringify(describeFeature(feature)))),
        symbol: { package: { ...storage, version } },
        capabilities: [],
    };
}

/** Declare a feature of the storage package. */
function declare(options: FeatureOptions): Feature {
    return defineFeature(options, { package: storage });
}

/** Plan the steps from the released seats to a later declaration. */
function planSeats(after: Feature) {
    return compareFeature(entry(seats, "2026.9.0"), entry(after, "2026.10.0")).steps;
}

/** Declare the seats as a static feature of some values. */
function staticSeats(value: schema.Schema): Feature {
    return declare({ ...seats.definition, kind: "static", value });
}

test("plan a feature's change between two releases", async () => {
    // accept an unchanged feature, widen its values safely, and refuse a narrowing or a change of kind
    expect([
        planSeats(seats),
        planSeats(staticSeats(schema.number().int().min(0))),
        await refusal(() => planSeats(staticSeats(schema.number().int().min(2)))),
        await refusal(() =>
            planSeats(declare({ name: seats.name, description: t`Seats.`, kind: "boolean" })),
        ),
    ]).toEqual([
        [],
        [{ action: "update", target: "feature/seats", risk: "safe", detail: "wider values" }],
        [
            "PRECONDITION_FAILED",
            "PLAN_BLOCKED",
            "cannot reach the desired state: feature/seats: declare a conversion for 2026.10.0",
        ],
        [
            "PRECONDITION_FAILED",
            "PLAN_BLOCKED",
            "cannot reach the desired state: feature/seats: stay at kind static, or declare a feature of kind boolean",
        ],
    ]);
});
