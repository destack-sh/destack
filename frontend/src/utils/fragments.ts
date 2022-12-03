import { graphql } from "@/gql";

export const ProjectVersionHeaderFragment = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    name
    description
    createdAt
    committed
    committedAt
  }
`);

export const FileHeaderFragment = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    path
    createdAt
    updatedAt
  }
`);
