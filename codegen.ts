import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "schema.gen.graphql",
  documents: ["bench-web/src/**/*.vue", "bench-web/src/**/*.ts"],
  ignoreNoDocuments: true,
  hooks: { afterOneFileWrite: ["prettier --write"] },
  generates: {
    "bench-web/src/gql/": {
      preset: "client",
      config: {
        useTypeImports: true,
        dedupeFragments: true,
        withCompositionFunctions: true,
      },
      plugins: [],
    },
  },
};

export default config;
