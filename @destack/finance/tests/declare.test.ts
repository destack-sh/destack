import { expect, test } from "@destack/test";
import { defineFeature, defineMeter } from "../src/declare/index.ts";
import {
    compareFeature,
    compareMeter,
    describeFeature,
    describeMeter,
    featureSymbols,
    featureVocabulary,
    meterVocabulary,
} from "../src/inspect/index.ts";
import { schema } from "@destack/schema";
import { FeatureCatalog, type Feature, type FeatureDefinition } from "../src/feature/index.ts";
import { release } from "./fixture/release.ts";
import {
    calls,
    quota,
    requests,
    seats,
    storage,
    stored,
    support,
    sync,
} from "./fixture/storage.ts";

test("read described features and meters back from a build, and relate metered features to their meters", async () => {
    // read the storage package's declarations back from its build
    const catalog = await FeatureCatalog.read(
        await release(storage, [requests, stored, sync, support, seats, calls, quota]),
    );
    const features = [sync, support, seats, calls, quota];
    expect([
        features.map((feature) => describeFeature(catalog.feature(feature.reference))),
        catalog.meter(requests.reference).definition,
    ]).toEqual([features.map((feature) => describeFeature(feature)), requests.definition]);

    // relate the metered feature to its meter, and the others to nothing
    expect([featureSymbols(describeFeature(calls)), featureSymbols(describeFeature(sync))]).toEqual(
        [
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
        ],
    );

    // refuse an undeclared feature
    expect(() => catalog.feature({ packageId: storage.id, name: "backup" })).toThrow(
        `feature ${storage.id}/backup is not declared`,
    );
});

/** Read a feature's release entry back from JSON. */
function entry(feature: Feature, version: string) {
    return {
        description: schema
            .record(schema.string(), schema.json())
            .parse(JSON.parse(JSON.stringify(describeFeature(feature)))),
        symbol: { package: { ...storage, version } },
    };
}

/** Declare a feature of the storage package. */
function declare(definition: FeatureDefinition): Feature {
    return defineFeature(definition, { package: storage });
}

test("plan a feature's change between releases: keep its kind, and accept the values earlier grants set", () => {
    const outcome = (after: Feature) => {
        try {
            return compareFeature(entry(seats, "2026.9.0"), entry(after, "2026.10.0")).steps;
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };

    // keep an unchanged feature, widen its values safely, and refuse a narrowing or a change of kind
    const values = (value: schema.Schema) =>
        declare({ ...seats.definition, kind: "static", value });
    expect([
        outcome(seats),
        outcome(values(schema.number().int().min(0))),
        outcome(values(schema.number().int().min(2))),
        outcome(declare({ name: seats.name, description: "Seats.", kind: "boolean" })),
    ]).toEqual([
        [],
        [{ action: "update", target: "feature/seats", risk: "safe", detail: "wider values" }],
        "feature/seats: declare a conversion for 2026.10.0",
        "feature/seats: keep kind static, or declare a feature of kind boolean",
    ]);
});

/** Read a description back from JSON, as a release keeps it. */
function readBack(description: unknown) {
    return schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(description)));
}

/** Describe the requests meter counting another way or in another unit. */
function counting(aggregation: "sum" | "max", unit: string) {
    return readBack(
        describeMeter(
            defineMeter({ ...requests.definition, aggregation, unit }, { package: storage }),
        ),
    );
}

test("plan a meter's change between releases: keep its aggregation and unit", () => {
    const outcome = (aggregation: "sum" | "max", unit: string) => {
        try {
            return compareMeter(
                { description: counting("sum", "request"), symbol: { package: storage } },
                {
                    description: counting(aggregation, unit),
                    symbol: { package: { ...storage, version: "2026.10.0" } },
                },
            ).steps;
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };

    // keep an unchanged meter, and refuse counting another way or in another unit
    expect([outcome("sum", "request"), outcome("max", "call")]).toEqual([
        [],
        "meter/api.requests: keep aggregation sum, or declare a meter of aggregation max; meter/api.requests: keep unit request, or declare a meter of unit call",
    ]);
});

test("name features and meters as terms of the package's vocabulary by what they mean", () => {
    // read the terms of a static and a metered feature and of a meter
    const terms = [seats, calls].map((feature) =>
        featureVocabulary(readBack(describeFeature(feature))),
    );
    const counted = meterVocabulary(readBack(describeMeter(requests)));

    expect([...terms, counted]).toEqual([
        { "feature/seats": { kind: "static" } },
        { "feature/api.calls": { kind: "metered", meter: `${storage.id}/api.requests` } },
        { "meter/api.requests": { aggregation: "sum", unit: "request" } },
    ]);
});
