/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n":
    types.DatasetContentFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...InstructionContent\n            ...DatasetContent\n          }\n        }\n      }\n    }\n  ":
    types.GetFileByIdDocument,
  "\n  fragment InstructionContent on Instruction {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n":
    types.InstructionContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  ":
    types.GetProjectBySlugDocument,
  "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionHeader\n        }\n        versions {\n          id\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.GetProjectVersionsDocument,
  "\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionHeader\n        files {\n          id\n          name\n          createdAt\n          updatedAt\n        }\n      }\n    }\n  ":
    types.GetProjectVersionFilesDocument,
};

export function graphql(
  source: "\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n"
): typeof documents["\n  fragment DatasetContent on Dataset {\n    id\n    records {\n      data\n      index\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"];
export function graphql(
  source: "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...InstructionContent\n            ...DatasetContent\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ...InstructionContent\n            ...DatasetContent\n          }\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment InstructionContent on Instruction {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n"
): typeof documents["\n  fragment InstructionContent on Instruction {\n    id\n    builtinId\n    code\n    parameters {\n      name\n      type\n      schema\n    }\n    arguments {\n      name\n      type\n      value\n      reference {\n        id\n        nameDotType\n      }\n    }\n  }\n"];
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
  source: "\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionHeader\n        files {\n          id\n          name\n          createdAt\n          updatedAt\n        }\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionHeader\n        files {\n          id\n          name\n          createdAt\n          updatedAt\n        }\n      }\n    }\n  "];

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
