import { DeclarationReference } from "@destack/package/declare";
import { graph } from "@destack/package";
import { Address, Plan, type Comparator } from "@destack/resource";
import { unreachablePlan } from "@destack/resource/error";
import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import { Feature } from "../feature/feature.ts";
import { FeatureDescription } from "../feature/feature.ts";

/** Describe a feature with its resource and its value's JSON Schema or its meters. */
export function describeFeature(feature: Feature): FeatureDescription {
    // describe the declaring package, name, description and resource
    const definition = feature.definition;
    const described = {
        package: feature.package,
        name: feature.name,
        description: definition.description,
        ...(definition.resource === undefined ? {} : { resource: definition.resource }),
    };

    // describe each kind with what its grants set
    if (definition.kind === "static") {
        const value = JsonValue.object(toJsonSchema(definition.value));

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

/** Plan a feature's change between releases: same kind, earlier grant values still valid. */
export const compareFeature: Comparator = (before, after) => {
    // read both releases' features
    const earlier = FeatureDescription.parse(before.description);
    const later = FeatureDescription.parse(after.description);
    const target = Address.join("feature", later.name);

    // refuse a change of a feature's kind
    if (earlier.kind !== later.kind) {
        throw unreachablePlan([
            {
                target,
                detail: `stay at kind ${earlier.kind}, or declare a feature of kind ${later.kind}`,
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
export function featureVocabulary(declared: Feature): Record<string, JsonValue> {
    const feature = describeFeature(declared);
    const term = Address.join("feature", feature.name);

    return {
        [term]:
            feature.kind === "metered"
                ? {
                      kind: feature.kind,
                      meters: feature.meters.map((meter) => DeclarationReference.key(meter)),
                  }
                : { kind: feature.kind },
    };
}

/** Read the meters a metered feature counts its usage with. */
export function featureSymbols(declared: Feature): graph.MemberSymbol[] {
    const feature = describeFeature(declared);
    const relationships =
        feature.kind === "metered"
            ? feature.meters.map((meter) => ({
                  kind: "reads",
                  symbol: { kind: "meter", name: meter.name, packageId: meter.packageId },
              }))
            : [];

    return schema.array(graph.MemberSymbol).parse([{ relationships }]);
}
