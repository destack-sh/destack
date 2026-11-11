import { fromJsonSchema, type JsonObject, JsonValue, schema, toJsonSchema } from "@destack/schema";
import type { Declaration } from "../graph/declaration.ts";
import type { BuildReader } from "../manifest/reader.ts";
import { DeclarationName, PackageId } from "./package.ts";
import { ModuleMetadata } from "./metadata.ts";

/** The state of a provisioned kind without a desired state: the empty object. */
export const NO_STATE = schema.object({});

/** Whether Destack creates a kind's resources, or they live outside it, as third-party APIs do. */
export const ResourceManagement = schema.enum(["internal", "external"]);
/** Whether Destack creates a provisioned kind's resources, or they live outside it. */
export type ResourceManagement = schema.Infer<typeof ResourceManagement>;

/** A kind as JSON: its package and name, with its description schema or its provisioning. */
export const KIND_JSON = schema.union([
    schema.object({
        /** The defining package. */
        packageId: PackageId,
        /** The kind's name within its package. */
        name: DeclarationName,
        /** The schema of its declarations' descriptions, as JSON Schema. */
        description: schema.jsonObject(),
    }),
    schema.object({
        /** The defining package. */
        packageId: PackageId,
        /** The kind's name within its package. */
        name: DeclarationName,
        /** How providers provision its declarations, which sets their descriptions. */
        provisioned: schema.object({
            /** The schema of a declaration's specification, as JSON Schema. */
            spec: schema.jsonObject(),
            /** The schema of the desired state, as JSON Schema, null without one. */
            state: schema.jsonObject().nullable(),
            /** Whether Destack creates the resources, or they live outside it. */
            management: ResourceManagement,
        }),
    }),
]);

/** How providers provision a kind's declarations: their spec, desired state and management. */
export interface Provisioning<
    Spec extends schema.Schema<JsonValue> = schema.Schema<JsonValue>,
    State extends schema.Schema = schema.Schema,
> {
    /** The schema of a declaration's specification. */
    readonly spec: Spec;
    /** The schema of the desired state providers reconcile toward, `NO_STATE` without one. */
    readonly state: State;
    /** Whether Destack creates the resources, or they live outside it. */
    readonly management: ResourceManagement;
}

/** The description of a provisioned kind's declaration: its name and its spec. */
export function provisionedDescription<Spec extends schema.Schema<JsonValue>>(spec: Spec) {
    return schema.object({
        /** The package-local name. */
        name: DeclarationName,
        /** The specification. */
        spec,
    });
}

/** A declaration kind: its package, its name, its descriptions' schema and its provisioning. */
export class Kind<
    Description extends schema.Schema = schema.Schema,
    Provisioned extends Provisioning | undefined = Provisioning | undefined,
> {
    /** The package defining the kind. */
    readonly packageId: PackageId;
    /** The kind's name within its package. */
    readonly name: DeclarationName;
    /** The schema of the kind's declarations' descriptions. */
    readonly description: Description;
    /** How providers provision the kind's declarations, absent for a kind nothing provisions. */
    readonly provisioned: Provisioned;
    /** The declarations of this kind each read build declares, parsed once, by reader. */
    readonly #read = new WeakMap<BuildReader, Promise<Declaration<schema.Output<Description>>[]>>();

    /** Name a kind of a package, with its descriptions' schema and provisioning. */
    constructor(
        packageId: string,
        name: string,
        description: Description,
        provisioned: Provisioned,
    ) {
        // validate the package and name, keeping the schemas
        this.packageId = PackageId.parse(packageId);
        this.name = DeclarationName.parse(name);
        this.description = description;
        this.provisioned = provisioned;
    }

    /** Describe the kind as JSON: its package, name, schemas and provisioning. */
    toJSON(): JsonObject {
        // write a declaration kind's description, or a provisioned kind's provisioning
        const { packageId, name, provisioned } = this;
        if (provisioned === undefined) {
            return {
                packageId,
                name,
                description: JsonValue.object(toJsonSchema(this.description)),
            };
        }
        const state = this.isStateful() ? JsonValue.object(toJsonSchema(provisioned.state)) : null;

        return {
            packageId,
            name,
            provisioned: {
                spec: JsonValue.object(toJsonSchema(provisioned.spec)),
                state,
                management: provisioned.management,
            },
        };
    }

    /** Rebuild a kind its JSON describes. */
    static fromJSON(value: unknown): Kind {
        // read the package, name and schemas, and the provisioning of a provisioned kind
        const parsed = KIND_JSON.parse(value);
        if ("description" in parsed) {
            return new Kind(
                parsed.packageId,
                parsed.name,
                fromJsonSchema(parsed.description),
                undefined,
            );
        }
        const { packageId, name, provisioned } = parsed;
        const spec = fromJsonSchema(provisioned.spec);
        const state = provisioned.state === null ? NO_STATE : fromJsonSchema(provisioned.state);

        return new Kind(packageId, name, provisionedDescription(spec), {
            spec,
            state,
            management: provisioned.management,
        });
    }

    /** Join a kind's package and name into one key, such as `@destack/db:database`. */
    static key(packageId: string, name: string): string {
        return `${packageId}:${name}`;
    }

    /** Report whether providers provision the kind's declarations. */
    isProvisioned(): this is Kind<Description, Provisioning> {
        return this.provisioned !== undefined;
    }

    /** Report whether providers reconcile the kind's declarations toward a desired state. */
    isStateful(): boolean {
        return this.provisioned !== undefined && this.provisioned.state !== NO_STATE;
    }

    /** The fields a declaration of this kind carries: its kind's name and package. */
    get identity(): { readonly kind: DeclarationName; readonly kindPackageId: PackageId } {
        return { kind: this.name, kindPackageId: this.packageId };
    }

    /** Read a build's declarations of this kind, parsed by its schema once per reader. */
    read(build: BuildReader): Promise<Declaration<schema.Output<Description>>[]> {
        // reuse a reader's parse, as its build never changes
        const known = this.#read.get(build);
        if (known !== undefined) {
            return known;
        }

        // parse once, forgetting a failed parse
        const reading = build.declarations().then((declarations) => this.select(declarations));
        this.#read.set(build, reading);
        reading.catch(() => this.#read.delete(build));

        return reading;
    }

    /** Select this kind's declarations, their descriptions parsed by its schema. */
    select<Selected extends Pick<Declaration, "kind" | "kindPackageId" | "description">>(
        declarations: readonly Selected[],
    ): (Omit<Selected, "description"> & { readonly description: schema.Output<Description> })[] {
        return declarations
            .filter((declaration) => this.has(declaration))
            .map((declaration) => ({
                ...declaration,
                description: this.description.parse(declaration.description),
            }));
    }

    /** Report whether a declaration is of this kind. */
    has(declaration: { readonly kind: string; readonly kindPackageId: string }): boolean {
        return this.is(declaration.kind, declaration.kindPackageId);
    }

    /** Report whether a kind's name and its defining package's id are this kind. */
    is(kind: string, packageId: string): boolean {
        return packageId === this.packageId && kind === this.name;
    }
}

/** Define a kind of the calling package by its declarations' descriptions' schema. */
export function defineKind<Description extends schema.Schema>(
    options: {
        /** The kind's name within its package. */
        readonly name: string;
        /** The schema of its declarations' descriptions. */
        readonly description: Description;
    },
    module?: ModuleMetadata,
): Kind<Description, undefined>;
/** Define a kind of the calling package whose declarations providers provision. */
export function defineKind<
    Spec extends schema.Schema<JsonValue>,
    State extends schema.Schema = typeof NO_STATE,
>(
    options: {
        /** The kind's name within its package, such as bucket. */
        readonly name: string;
        /** How providers provision its declarations. */
        readonly provisioned: {
            readonly spec: Spec;
            readonly state?: State;
            readonly management?: ResourceManagement;
        };
    },
    module?: ModuleMetadata,
): Kind<ReturnType<typeof provisionedDescription<Spec>>, Provisioning<Spec, State>>;
/** Define a kind of the calling package, as one of the signatures above. */
export function defineKind(
    options:
        | { readonly name: string; readonly description: schema.Schema }
        | {
              readonly name: string;
              readonly provisioned: {
                  readonly spec: schema.Schema<JsonValue>;
                  readonly state?: schema.Schema;
                  readonly management?: ResourceManagement;
              };
          },
    module?: ModuleMetadata,
): Kind {
    // stamp the declaring package, and describe a provisioned kind's declarations by name and spec
    const owner = ModuleMetadata.require(module, "defineKind").package;
    if ("provisioned" in options) {
        const { spec, state, management } = options.provisioned;
        const provisioning = {
            spec,
            state: state ?? NO_STATE,
            management: management ?? "internal",
        };

        return new Kind(owner.id, options.name, provisionedDescription(spec), provisioning);
    }

    return new Kind(owner.id, options.name, options.description, undefined);
}
