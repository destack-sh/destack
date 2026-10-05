import { graph } from "@destack/package";
import { Address, Plan, type Comparator } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { schema, toJsonSchema, type JsonObject } from "@destack/schema";
import { Feature } from "../feature/feature.ts";
import { FeatureDescription } from "../feature/description.ts";

/** Describe a feature with its value's JSON Schema or its meter. */
export function describeFeature(feature: Feature): FeatureDescription {
    // describe the declaring package, name and description
    const definition = feature.definition;
    const described = {
        package: feature.package,
        name: feature.name,
        description: definition.description,
    };

    // describe each kind with what its grants set
    if (definition.kind === "static") {
        const value = schema
            .record(schema.string(), schema.json())
            .parse(toJsonSchema(definition.value));

        return { ...described, kind: "static", value };
    } else if (definition.kind === "metered") {
        return { ...described, kind: "metered", meter: definition.meter, reset: definition.reset };
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

/** Read the meter a metered feature counts its usage with. */
export function featureSymbols(input: JsonObject): graph.MemberSymbol[] {
    const feature = FeatureDescription.parse(input);
    const relationships =
        feature.kind === "metered"
            ? [
                  {
                      kind: "reads",
                      symbol: {
                          kind: "meter",
                          name: feature.meter.name,
                          packageId: feature.meter.packageId,
                      },
                  },
              ]
            : [];

    return schema.array(graph.MemberSymbol).parse([{ relationships }]);
}
