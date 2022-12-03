import { graphql } from "@/gql";

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    path
    createdAt
    updatedAt
  }
`);

export const CompilationHeaderType = graphql(/* GraphQL */ `
  fragment CompilationHeader on Compilation {
    id
    name
    createdAt
    updatedAt
    task {
      id
      nameDotType
    }
    backends {
      id
      nameDotType
    }
    targetTask {
      id
      nameDotType
    }
    targetCode {
      id
      nameDotType
    }
  }
`);
