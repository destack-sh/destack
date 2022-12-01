/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        name\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ... on Task {\n              schema\n              expectations {\n                nameDotType\n              }\n              templateImplementation {\n                nameDotType\n              }\n              compilations {\n                name\n              }\n            }\n            ... on Dataset {\n              records {\n                data\n                index\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.GetFileByIdDocument,
  "\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n":
    types.ProjectVersionFragmentFragmentDoc,
  "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  ":
    types.GetProjectBySlugDocument,
  "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  ":
    types.GetProjectVersionsDocument,
  "\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        files {\n          id\n          name\n        }\n      }\n    }\n  ":
    types.GetProjectVersionFilesDocument,
};

export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"];
export function graphql(
  source: "\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        name\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ... on Task {\n              schema\n              expectations {\n                nameDotType\n              }\n              templateImplementation {\n                nameDotType\n              }\n              compilations {\n                name\n              }\n            }\n            ... on Dataset {\n              records {\n                data\n                index\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query getFileById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        name\n        definitions {\n          id\n          name\n          type\n          nameDotType\n          createdAt\n          updatedAt\n          content {\n            ... on Task {\n              schema\n              expectations {\n                nameDotType\n              }\n              templateImplementation {\n                nameDotType\n              }\n              compilations {\n                name\n              }\n            }\n            ... on Dataset {\n              records {\n                data\n                index\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n"
): typeof documents["\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n"];
export function graphql(
  source: "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "
): typeof documents["\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "];
export function graphql(
  source: "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        files {\n          id\n          name\n        }\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersionFiles($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        files {\n          id\n          name\n        }\n      }\n    }\n  "];

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
