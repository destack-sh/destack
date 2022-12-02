/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n":
    types.CodeContentFragmentDoc,
  "\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n":
    types.DatasetContentFragmentDoc,
  "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      nameDotType\n    }\n  }\n":
    types.ExpectationContentFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...CodeContent\n            ...DatasetContent\n            ...ExpectationContent\n            ...TaskContent\n          }\n        }\n      }\n    }\n  ":
    types.GetFileByIdDocument,
  "\n  fragment TaskContent on Task {\n    id\n    schema\n    expectations {\n      id\n      description\n      nameDotType\n    }\n    templateImplementation {\n      id\n      nameDotType\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n":
    types.TaskContentFragmentDoc,
  "\n  fragment CompilationHeader on Compilation {\n    name\n    createdAt\n    updatedAt\n    backends {\n      id\n      nameDotType\n    }\n    targetTask {\n      id\n      nameDotType\n    }\n    targetCode {\n      id\n      nameDotType\n    }\n  }\n":
    types.CompilationHeaderFragmentDoc,
  "\n    mutation compileTask($compilationId: UUID!) {\n      compile(compilationId: $compilationId)\n    }\n  ":
    types.CompileTaskDocument,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  ":
    types.GetProjectBySlugDocument,
  "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionHeader\n        }\n        versions {\n          id\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.GetProjectVersionsDocument,
  "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    # mainProgram {\n    #   id\n    #   name\n    #   type\n    #   nameDotType\n    #   content {\n    #     ...TaskContent\n    #   }\n    # }\n    files {\n      id\n      ...FileHeader\n    }\n  }\n":
    types.ProjectVersionContentFragmentDoc,
  "\n    query getProjectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  ":
    types.GetProjectVersionContentDocument,
};

export function graphql(
  source: "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n"
): typeof documents["\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n"
): typeof documents["\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      nameDotType\n    }\n  }\n"
): typeof documents["\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      nameDotType\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n"];
export function graphql(
  source: "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...CodeContent\n            ...DatasetContent\n            ...ExpectationContent\n            ...TaskContent\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...CodeContent\n            ...DatasetContent\n            ...ExpectationContent\n            ...TaskContent\n          }\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment TaskContent on Task {\n    id\n    schema\n    expectations {\n      id\n      description\n      nameDotType\n    }\n    templateImplementation {\n      id\n      nameDotType\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"
): typeof documents["\n  fragment TaskContent on Task {\n    id\n    schema\n    expectations {\n      id\n      description\n      nameDotType\n    }\n    templateImplementation {\n      id\n      nameDotType\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment CompilationHeader on Compilation {\n    name\n    createdAt\n    updatedAt\n    backends {\n      id\n      nameDotType\n    }\n    targetTask {\n      id\n      nameDotType\n    }\n    targetCode {\n      id\n      nameDotType\n    }\n  }\n"
): typeof documents["\n  fragment CompilationHeader on Compilation {\n    name\n    createdAt\n    updatedAt\n    backends {\n      id\n      nameDotType\n    }\n    targetTask {\n      id\n      nameDotType\n    }\n    targetCode {\n      id\n      nameDotType\n    }\n  }\n"];
export function graphql(
  source: "\n    mutation compileTask($compilationId: UUID!) {\n      compile(compilationId: $compilationId)\n    }\n  "
): typeof documents["\n    mutation compileTask($compilationId: UUID!) {\n      compile(compilationId: $compilationId)\n    }\n  "];
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n"];
export function graphql(
  source: "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "
): typeof documents["\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "];
export function graphql(
  source: "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionHeader\n        }\n        versions {\n          id\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionHeader\n        }\n        versions {\n          id\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    # mainProgram {\n    #   id\n    #   name\n    #   type\n    #   nameDotType\n    #   content {\n    #     ...TaskContent\n    #   }\n    # }\n    files {\n      id\n      ...FileHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    # mainProgram {\n    #   id\n    #   name\n    #   type\n    #   nameDotType\n    #   content {\n    #     ...TaskContent\n    #   }\n    # }\n    files {\n      id\n      ...FileHeader\n    }\n  }\n"];
export function graphql(
  source: "\n    query getProjectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "];

export function graphql(source: string): unknown;
export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<
  infer TType,
  any
>
  ? TType
  : never;
