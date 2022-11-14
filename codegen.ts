import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "./schema.gen.graphql",
  documents: ["frontend/src/**/*.vue", "frontend/src/**/*.ts"],
  ignoreNoDocuments: true,
  hooks: { afterOneFileWrite: ["prettier --write"] },
  generates: {
    "./frontend/src/gql/": {
      preset: "client",
      config: {
        useTypeImports: true,
        withCompositionFunctions: true,
      },
      plugins: ["typescript-vue-apollo"],
    },
    // below: alternative config if we want to put operations near their definitions
    // "frontend/src/gql/types.ts": {
    //   config: {
    //     useTypeImports: true,
    //     withCompositionFunctions: true,
    //   },
    //   plugins: ["typescript"],
    // },
    // "frontend/src/": {
    //   preset: "near-operation-file",
    //   presetConfig: {
    //     extension: ".gql.ts",
    //     baseTypesPath: "gql/types.ts",
    //   },
    //   plugins: ["typescript-operations", "typescript-vue-apollo"],
    // },
  },
};

export default config;
