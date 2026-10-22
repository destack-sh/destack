import {
    type Condition,
    type DatabaseConnection,
    eq,
    type RowImage,
    type Snapshot,
    sql,
} from "@destack/db";
import { type Duration, schema } from "@destack/schema";
import { replica, Subject, type Subscription } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import { QUERIES_PUBLICATION, type ObjectQuery } from "./replica.ts";

/** A shape's key: the scope whose rows it reads, and the parameters its queries take. */
export type ShapeKey<Parameters extends object> = {
    readonly [Name in keyof Parameters as string extends Name ? never : Name]: Parameters[Name];
} & {
    /** The scope the rows live in, whose home publishes them. */
    readonly scope: string;
};

/** A named, keyed set of live object queries: a reified sync query a package's code follows. */
export class Shape<Parameters extends object = object> {
    /** The shape as its package declares it. */
    readonly #definition: ShapeDefinition<Parameters>;

    /** Create a shape as its package declares it. */
    constructor(definition: ShapeDefinition<Parameters>) {
        this.#definition = definition;
    }

    /** The shape's name, which its replicas' names start with. */
    get name(): string {
        return this.#definition.name;
    }

    /** The schema of the parameters its queries take beside the scope. */
    get parameters(): schema.Schema<Parameters> {
        return this.#definition.parameters;
    }

    /** The object types its queries read, which every follower knows. */
    get objects(): readonly ObjectType[] {
        return this.#definition.objects;
    }

    /** Write the shape's queries for a key's parameters. */
    queries(parameters: Parameters): readonly ObjectQuery[] {
        return this.#definition.queries(parameters);
    }

    /** Build the replica name of a key: the one writer of replica names. */
    replicaName(key: ShapeKey<Parameters>): string {
        // join the scope and the parameters' values in their names' order
        const { scope, ...parameters } = key;

        return [this.name, scope, ...Shape.#values(parameters)].join(":");
    }

    /** List the replicas a database has of the shape for some parameters, at whatever scope. */
    async replicas(
        database: DatabaseConnection,
        parameters: Parameters,
    ): Promise<{ readonly name: string; readonly scope: string }[]> {
        // match the names the scope of each record completes
        const suffix = Shape.#values(parameters)
            .map((value) => `:${value}`)
            .join("");

        return await database
            .select({ name: replica.name, scope: replica.scope })
            .from(replica)
            .where(eq(replica.name, sql`${`${this.name}:`} || ${replica.scope} || ${suffix}`));
    }

    /** Subscribe to a key for a follower, on behalf of a principal if given. */
    subscription(key: ShapeKey<Parameters>, target: string, subject?: Subject): Subscription {
        // label each query of the key's parameters by its type and position
        const { scope, ...rest } = key;
        const named = this.#named(this.parameters.parse(rest), "");

        return {
            name: this.replicaName(key),
            publication: QUERIES_PUBLICATION,
            scope,
            target,
            ...(subject === undefined ? {} : { subject }),
            parameters: { queries: schema.json().parse(named) },
        };
    }

    /** Subscribe to the keys a follower acquired at one scope, as one replica. */
    onDemandSubscription(
        scope: string,
        keys: readonly Parameters[],
        target: string,
        subject?: Subject,
    ): Subscription {
        // label each key's queries by its parameter values, its type and its position
        const named = Object.fromEntries(
            keys.flatMap((parameters) =>
                Object.entries(this.#named(parameters, `${Shape.#values(parameters).join(":")}:`)),
            ),
        );

        return {
            name: [this.name, scope, ...(subject === undefined ? [] : [Subject.key(subject)])].join(
                ":",
            ),
            publication: QUERIES_PUBLICATION,
            scope,
            target,
            ...(subject === undefined ? {} : { subject }),
            parameters: { queries: schema.json().parse(named) },
        };
    }

    /** Name the queries of some parameters by a prefix, their type and position. */
    #named(parameters: Parameters, prefix: string): Record<string, ObjectQuery> {
        // refuse a query of a type the shape does not list
        const queries = this.queries(parameters);
        const stray = queries.find(
            (query) => !this.objects.some((object) => object.name === query.object),
        );
        if (stray !== undefined) {
            throw new TypeError(
                `shape ${this.name} queries ${stray.object}, which it does not list`,
            );
        }

        return Object.fromEntries(
            queries.map((query, index) => [`${prefix}${index}:${query.object}`, query]),
        );
    }

    /** List a key's parameter values in their names' order. */
    static #values(parameters: object): string[] {
        return Object.entries(parameters)
            .toSorted(([left], [right]) => left.localeCompare(right))
            .map(([, value]) => String(value));
    }
}

/** A shape as its package declares it: its name, key parameters, object types and queries. */
interface ShapeDefinition<Parameters extends object> {
    /** The shape's name. */
    readonly name: string;
    /** The schema of its parameters beside the scope, an empty object for a shape without any. */
    readonly parameters: schema.Schema<Parameters>;
    /** The object types its queries read. */
    readonly objects: readonly ObjectType[];
    /** Write the queries for a key's parameters. */
    queries(parameters: Parameters): readonly ObjectQuery[];
}

/** A key a store follows a shape at, and the principal its replica is decided for. */
export interface FollowKey<Parameters extends object = object> {
    /** The shape's key. */
    readonly key: ShapeKey<Parameters>;
    /** The principal the replica is decided for, the store's principal when absent. */
    readonly subject?: Subject;
}

/** A shape a store reads replicated: at fixed keys, per local row, on demand, or down its chain. */
export type Follow<Parameters extends object = object, From extends ObjectType = ObjectType> =
    | {
          /** The followed shape. */
          readonly shape: Shape<Parameters>;
          /** The keys it follows the shape at. */
          readonly keys: readonly FollowKey<Parameters>[];
      }
    | {
          /** The followed shape. */
          readonly shape: Shape<Parameters>;
          /** The object type of the follower's rows its keys come from. */
          readonly from: From;
          /** The condition on those rows, every row when absent. */
          readonly where?: Condition<RowImage<From["table"]>>;
          /** Read a row's key as the follower, absent for a row it follows nothing for. */
          key(
              row: RowImage<From["table"]>,
              view: Snapshot,
              space: string | undefined,
          ): Promise<FollowKey<Parameters> | undefined>;
      }
    | {
          /** The followed shape. */
          readonly shape: Shape<Parameters>;
          /** How long a key the store acquires stays followed after its last acquisition. */
          readonly onDemand: { readonly ttl: Duration };
      }
    | {
          /** The types the scopes above the store hand down to it, down its chain. */
          readonly chain: readonly ObjectType[];
      };

/** A follow of a shape at one key per row of the follower, of one object type. */
export type RowFollow<Parameters extends object, From extends ObjectType> = Extract<
    Follow<Parameters, From>,
    { readonly from: unknown }
>;

/** Follows of shapes. */
export const Follow = {
    /** Follow the handed-down rows of some inherited types down the store's chain. */
    chain(types: readonly ObjectType[]): Follow {
        return { chain: types };
    },

    /** List the object types a follow replicates: its shape's, or its chain's. */
    types(follow: Follow): readonly ObjectType[] {
        return "chain" in follow ? follow.chain : follow.shape.objects;
    },

    /** Follow a shape at fixed keys or on demand. */
    of<
        Parameters extends object,
        const At extends DistributiveOmit<
            Exclude<Follow<Parameters>, { readonly from: unknown } | { readonly chain: unknown }>,
            "shape"
        >,
    >(shape: Shape<Parameters>, at: At): NoInfer<At & { readonly shape: Shape<Parameters> }> {
        return { ...at, shape };
    },
};

/** Omit keys from each member of a union. */
type DistributiveOmit<Union, Key extends PropertyKey> = Union extends unknown
    ? Omit<Union, Key>
    : never;

/** Declare a shape: its name, the parameters of its key, and the object queries they write. */
export function defineShape<Parameters extends object>(
    definition: ShapeDefinition<Parameters>,
): Shape<Parameters> {
    return new Shape(definition);
}
