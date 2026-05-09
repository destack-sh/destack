import assert from "assert";
import { readFileSync } from "fs";
import { version, graphqlSync } from "graphql";
import { buildSchema } from "graphql/utilities";

assert.deepStrictEqual(
    version,
    JSON.parse(readFileSync("./node_modules/graphql/package.json")).version,
);

const schema = buildSchema("type Query { hello: String }");

const result = graphqlSync({
    schema,
    source: "{ hello }",
    rootValue: { hello: "world" },
});

assert.deepStrictEqual(result, {
    data: {
        __proto__: null,
        hello: "world",
    },
});
