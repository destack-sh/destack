import { graph } from "@destack/package";
import { Address, Plan, type Comparator } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { schema, toJsonSchema, type JsonObject, type JsonValue } from "@destack/schema";
import { Feature } from "../feature/feature.ts";
import { FeatureDescription } from "../feature/description.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** Describe a feature with its allowance and its value's JSON Schema or its meters. */
export function describeFeature(feature: Feature): FeatureDescription {
    // describe the declaring package, name, description and allowance
    const definition = feature.definition;
    const described = {
        package: feature.package,
        name: feature.name,
        description: definition.description,
        ...(definition.allowance === undefined ? {} : { allowance: definition.allowance }),
    };

    // describe each kind with what its grants set
    if (definition.kind === "static") {
        const value = schema
            .record(schema.string(), schema.json())
            .parse(toJsonSchema(definition.value));

        return { ...described, kind: "static", value };
    } else if (definition.kind === "metered") {
        return {
            ...described,
            kind: "metered",
            meters: [...definition.meters],
            reset: definition.reset,
        };
    }

    return { ...described, kind: "boolean" };
}

/** Plan a feature's change between releases: keep its kind, and accept every value earlier grants set. */
export const compareFeature: Comparator = (before, after) => {
    // read both releases' features
    const earlier = FeatureDescription.parse(before.description);
    const later = FeatureDescription.parse(after.description);
    const target = Address.join("feature", later.name);

    // refuse a change of kind, which kept grants and entitlements cannot follow
    if (earlier.kind !== later.kind) {
        throw new PlanError([
            {
                target,
                detail: `keep kind ${earlier.kind}, or declare a feature of kind ${later.kind}`,
            },
        ]);
    }
    // let a static feature accept the values earlier grants set
    else if (earlier.kind === "static" && later.kind === "static") {
        return Plan.values({
            target,
            before: earlier.value,
            after: later.value,
            release: after.symbol.package.version,
            compatibility: "backward",
            isConverted: false,
        });
    }

    return { steps: [] };
};

/** List a feature's term: its kind, and the meters a metered feature counts. */
export function featureVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    const feature = FeatureDescription.parse(input);
    const term = Address.join("feature", feature.name);

    return {
        [term]:
            feature.kind === "metered"
                ? {
                      kind: feature.kind,
                      meters: feature.meters.map((meter) => CatalogReference.key(meter)),
                  }
                : { kind: feature.kind },
    };
}

/** Read the meters a metered feature counts its usage with. */
export function featureSymbols(input: JsonObject): graph.MemberSymbol[] {
    const feature = FeatureDescription.parse(input);
    const relationships =
        feature.kind === "metered"
            ? feature.meters.map((meter) => ({
                  kind: "reads",
                  symbol: { kind: "meter", name: meter.name, packageId: meter.packageId },
              }))
            : [];

    return schema.array(graph.MemberSymbol).parse([{ relationships }]);
}
