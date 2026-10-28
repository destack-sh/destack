import { type Condition, type DatabaseConnection, type RowImage, type Snapshot } from "@destack/db";
import { canonicalize, type Duration, schema } from "@destack/schema";
import { Replica, Subject, type SubscriptionRecord } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import { QUERIES_PUBLICATION, QueriesParameters, type ObjectQuery } from "./replica.ts";

/** The name the replicas of a store's followed shapes start with. */
const FOLLOW_REPLICA = "follow";

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

    /** Build the name of the replica following the shapes' keys at a scope for a principal. */
    static replicaName(scope: string, subject?: Subject): string {
        return [
            FOLLOW_REPLICA,
            scope,
            ...(subject === undefined ? [] : [Subject.key(subject)]),
        ].join(":");
    }

    /** Subscribe a follower to some shapes' keys at one scope as one replica. */
    static subscription(
        scope: string,
        followed: readonly ShapeKeys[],
        target: string,
        subject?: Subject,
    ): SubscriptionRecord {
        // name each shape's keys' queries under the shape
        const queries = Object.fromEntries(
            followed.flatMap(({ shape, keys }) => Object.entries(shape.#keyed(keys))),
        );

        return {
            name: Shape.replicaName(scope, subject),
            publication: QUERIES_PUBLICATION,
            scope,
            target,
            ...(subject === undefined ? {} : { subject }),
            parameters: { queries: schema.json().parse(queries) },
        };
    }

    /** Report whether a database's synced replica follows a key's queries. */
    async isSynced(
        database: DatabaseConnection,
        key: ShapeKey<Parameters>,
        subject?: Subject,
    ): Promise<boolean> {
        // require a synced replica of the key's scope
        const { scope, ...rest } = key;
        const name = Shape.replicaName(scope, subject);
        const recorded = await Replica.subscription(database, name, scope);
        if (recorded === undefined || !(await Replica.isSynced(database, name, scope))) {
            return false;
        }

        // require each of the key's queries among the recorded ones
        const has = QueriesParameters.parse(recorded.parameters).queries ?? {};
        const wanted = this.#keyed([this.parameters.parse(rest)]);

        return Object.entries(wanted).every(
            ([named, query]) =>
                has[named] !== undefined && canonicalize(has[named]) === canonicalize(query),
        );
    }

    /** Name each key's queries by the shape, the key's values, its type and position. */
    #keyed(keys: readonly object[]): Record<string, ObjectQuery> {
        return Object.fromEntries(
            keys.flatMap((key) => {
                const parameters = this.parameters.parse(key);
                const prefix = [this.name, ...Shape.#values(parameters)].join(":");

                return Object.entries(this.#named(parameters, `${prefix}:`));
            }),
        );
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

/** Some keys of one shape a replica follows at one scope. */
export interface ShapeKeys {
    /** The shape. */
    readonly shape: Shape;
    /** The keys' parameters beside the scope. */
    readonly keys: readonly object[];
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
