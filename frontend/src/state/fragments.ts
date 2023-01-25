import { graphql } from "@/gql";

export const OperationInfoContentType = graphql(/* GraphQL */ `
  fragment OperationInfoContent on OperationInfo {
    ... on OperationInfo {
      messages {
        kind
        message
        field
      }
    }
  }
`);

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    parents {
      id
    }
  }
`);

export const ProjectHeaderType = graphql(/* GraphQL */ `
  fragment ProjectHeader on Project {
    id
    name
    slug
    createdAt
    updatedAt
    head {
      ...ProjectVersionHeader
    }
    organization {
      slug
    }
  }
`);

export const ProjectVersionContentType = graphql(/* GraphQL */ `
  fragment ProjectVersionContent on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    files(filters: { isVisible: true }) {
      id
      ...FileHeader
    }
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    revision
    name
    path
    createdAt
    updatedAt
    deletedAt
    projectVersion {
      id
    }
  }
`);

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    id
    type
    revision
    symbolType
    createdAt
    updatedAt
    deletedAt
    modifier
    name
    compiled
    commented
    index
    parent {
      id
    }
    reference {
      id
    }
  }
`);

export const TypeContentType = graphql(/* GraphQL */ `
  fragment TypeContent on Type {
    description
    btl
  }
`);

export const StatementContentType = graphql(/* GraphQL */ `
  fragment StatementContent on Statement {
    id
    type
    revision
    symbolType
    createdAt
    updatedAt
    deletedAt
    name
    commented
    compiled
    modifier
    index
    parent {
      id
    }
    reference {
      id
    }
    importPath
    text
    # symbol contents
    lang
    code
    codeBuiltinId
    description
    referenceProjectVersion {
      id
    }
    value
    btl
    records {
      data
    }
  }
`);
