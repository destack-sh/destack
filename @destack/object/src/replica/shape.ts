import { type DatabaseConnection, eq, type Snapshot, sql } from "@destack/db";
import { schema } from "@destack/schema";
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
export interface Shape<Parameters extends object = object> {
    /** The shape's name, which its replicas' names start with. */
    readonly name: string;
    /** The schema of the parameters its queries take beside the scope. */
    readonly parameters: schema.Schema<Parameters>;
    /** The object types its queries read, which every follower knows. */
    readonly objects: readonly ObjectType[];
    /** Write the shape's queries for a key's parameters. */
    queries(parameters: Parameters): readonly ObjectQuery[];
}

/** A key a store follows a shape at, and the principal its replica is decided for. */
export interface FollowKey<Parameters extends object = object> {
    /** The shape's key. */
    readonly key: ShapeKey<Parameters>;
    /** The principal the replica is decided for, the store's own when absent. */
    readonly subject?: Subject;
}

/** A shape a package's code reads replicated: at fixed keys, or at one key per row of its own. */
export type Follow<Parameters extends object = object> = {
    /** Whether the store waits for these replicas before serving, true by default. */
    readonly isReadinessGate?: boolean;
} & (
    | {
          /** The followed shape. */
          readonly shape: Shape<Parameters>;
          /** The keys it follows the shape at. */
          readonly keys: readonly FollowKey<Parameters>[];
      }
    | {
          /** The followed shape. */
          readonly shape: Shape<Parameters>;
          /** Write the query of the follower's own rows that give its keys. */
          readonly rows: (self: Subject) => ObjectQuery;
          /** Read a row's key as the follower, absent for a row it follows nothing for. */
          readonly key: (
              row: Readonly<Record<string, unknown>>,
              view: Snapshot,
              self: Subject,
          ) => Promise<FollowKey<Parameters> | undefined>;
      }
);

/** Follows of shapes. */
export const Follow = {
    /** Follow a shape at fixed keys, or at one key per row of the follower's own. */
    of<Parameters extends object>(
        shape: Shape<Parameters>,
        at: DistributiveOmit<Follow<Parameters>, "shape">,
    ): Follow<Parameters> {
        return { shape, ...at };
    },
};

/** Omit keys from each member of a union. */
type DistributiveOmit<Union, Key extends PropertyKey> = Union extends unknown
    ? Omit<Union, Key>
    : never;

/** Shapes and their replicas. */
export const Shape = {
    /** Name the replica of a shape's key: the one writer of replica names. */
    replicaName<Parameters extends object>(
        shape: Shape<Parameters>,
        key: ShapeKey<Parameters>,
    ): string {
        // join the scope and the parameters' values in their names' order
        const { scope, ...parameters } = key;

        return [shape.name, scope, ...Shape.listValues(parameters)].join(":");
    },

    /** List the replicas a database keeps of a shape for some parameters, at whatever scope. */
    async kept<Parameters extends object>(
        database: DatabaseConnection,
        shape: Shape<Parameters>,
        parameters: Parameters,
    ): Promise<{ readonly name: string; readonly scope: string }[]> {
        // match the names the scope of each record completes
        const suffix = Shape.listValues(parameters)
            .map((value) => `:${value}`)
            .join("");

        return await database
            .select({ name: replica.name, scope: replica.scope })
            .from(replica)
            .where(eq(replica.name, sql`${`${shape.name}:`} || ${replica.scope} || ${suffix}`));
    },

    /** Subscribe to a shape's key for a follower, on behalf of a principal if given. */
    subscription<Parameters extends object>(
        shape: Shape<Parameters>,
        key: ShapeKey<Parameters>,
        below: string,
        subject?: Subject,
    ): Subscription {
        // read the key's parameters, and name each query by its type and position
        const { scope, ...rest } = key;
        const parameters = shape.parameters.parse(rest);
        const queries = shape.queries(parameters);
        const stray = queries.find(
            (query) => !shape.objects.some((object) => object.name === query.object),
        );
        if (stray !== undefined) {
            throw new TypeError(
                `shape ${shape.name} queries ${stray.object}, which it does not list`,
            );
        }
        const named = Object.fromEntries(
            queries.map((query, index) => [`${index}:${query.object}`, query]),
        );

        return {
            name: Shape.replicaName(shape, key),
            publication: QUERIES_PUBLICATION,
            scope,
            below,
            ...(subject === undefined ? {} : { subject }),
            parameters: { queries: schema.json().parse(named) },
        };
    },

    /** List a key's parameter values in their names' order. */
    listValues(parameters: object): string[] {
        return Object.entries(parameters)
            .toSorted(([left], [right]) => left.localeCompare(right))
            .map(([, value]) => String(value));
    },
};

/** Declare a shape: its name, the parameters of its key, and the object queries they write. */
export function defineShape<Parameters extends object>(definition: {
    /** The shape's name. */
    readonly name: string;
    /** The schema of its parameters beside the scope, an empty object for a shape without any. */
    readonly parameters: schema.Schema<Parameters>;
    /** The object types its queries read. */
    readonly objects: readonly ObjectType[];
    /** Write the queries for a key's parameters. */
    readonly queries: (parameters: Parameters) => readonly ObjectQuery[];
}): Shape<Parameters> {
    return {
        name: definition.name,
        parameters: definition.parameters,
        objects: definition.objects,
        queries: (parameters) => definition.queries(parameters),
    };
}
