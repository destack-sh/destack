import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "./schema.gen.graphql",
  documents: ["frontend/src/**/*.vue", "frontend/src/**/*.ts"],
  ignoreNoDocuments: true,
  generates: {
    "./frontend/src/gql/": {
      preset: "client",
      config: {
        useTypeImports: true,
        withCompositionFunctions: true,
      },
      plugins: ["typescript", "typescript-operations", "typescript-vue-apollo"],
    },
  },
};

export default config;
