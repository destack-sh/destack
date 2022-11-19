/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n":
    types.ProjectVersionFragmentFragmentDoc,
  "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  ":
    types.GetProjectBySlugDocument,
  "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  ":
    types.GetProjectVersionsDocument,
};

export function graphql(
  source: "\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n"
): typeof documents["\n  fragment ProjectVersionFragment on ProjectVersion {\n    name\n    description\n    createdAt\n    committedAt\n  }\n"];
export function graphql(
  source: "\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "
): typeof documents["\n    query getProjectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        id\n      }\n    }\n  "];
export function graphql(
  source: "\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  "
): typeof documents["\n    query getProjectVersions($id: GlobalID!) {\n      project(id: $id) {\n        id\n        name\n        slug\n        head {\n          id\n          ...ProjectVersionFragment\n        }\n        versions {\n          id\n          ...ProjectVersionFragment\n        }\n      }\n    }\n  "];

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
