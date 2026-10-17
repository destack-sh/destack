import type { MethodContextOf } from "@destack/object";
import type {} from "@destack/stack/declare";
import type { price, product, productFeature, seller } from "../object/index.ts";

/** A record declared as its type's create method takes it, less what the stack supplies. */
type Declared<
    Type extends typeof seller | typeof product | typeof price | typeof productFeature,
    Supplied extends string,
> = Omit<MethodContextOf<Type, "create">["input"], "spaceId" | "id" | Supplied>;

/** Add the sellers and products collections to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The space account's seller, as its create method takes it. */
        readonly sellers?: Readonly<Record<string, Declared<typeof seller, "accountId">>>;
        /** The space account's products, each with its prices and feature grants by name. */
        readonly products?: Readonly<
            Record<
                string,
                Declared<typeof product, "accountId"> & {
                    /** The product's prices by name. */
                    readonly prices?: Readonly<Record<string, Declared<typeof price, "parentId">>>;
                    /** The product's feature grants by name. */
                    readonly productFeatures?: Readonly<
                        Record<string, Declared<typeof productFeature, "parentId">>
                    >;
                }
            >
        >;
    }
}
