import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "schema.gen.graphql",
  documents: ["frontend/src/**/*.vue", "frontend/src/**/*.ts"],
  ignoreNoDocuments: true,
  hooks: { afterOneFileWrite: ["prettier --write"] },
  generates: {
    "frontend/src/gql/": {
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
