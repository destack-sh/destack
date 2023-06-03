import { graphql } from "@/gql";

export const PageInfoType = graphql(/* GraphQL */ `
  fragment PageInfo on PageInfo {
    hasNextPage
    hasPreviousPage
    startCursor
    endCursor
  }
`);

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
    tag
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
    type
    visibility
    name
    slug
    createdAt
    updatedAt
    canWrite
    head {
      ...ProjectVersionHeader
    }
    owner {
      ... on Organization {
        id
        slug
        name
      }
      ... on User {
        id
        slug
        username
        name
      }
    }
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    revision
    name
    path
    parent {
      id
    }
    createdAt
    updatedAt
    deletedAt
    directory
    generated
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
    generated
    commented
    orderKey
    parent {
      id
    }
    reference {
      id
    }
  }
`);

export const FieldType = graphql(/* GraphQL */ `
  fragment FieldContent on Field {
    id
    createdAt
    updatedAt
    deletedAt
    revision
    name
    key
    tag
    hint
    description
    value
    orderKey
    reference {
      id
    }
    flags
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
    generated
    modifier
    orderKey
    parent {
      id
    }
    reference {
      id
    }
    # symbol contents
    lang
    code
    description
    referenceProjectVersion {
      id
    }
    rootTypeTag
    rootTypeFlags
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
    # interp
    resolvedFields {
      ...FieldContent
    }
    issues {
      ...IssueContent
    }
  }
`);

export const IssueContentType = graphql(/* GraphQL */ `
  fragment IssueContent on Issue {
    id
    kind
    type
    message
    statement {
      id
    }
  }
`);

export const InterpDataContentType = graphql(/* GraphQL */ `
  fragment InterpDataContent on InterpData {
    statementId
    issues {
      id
      kind
      type
      message
    }
    resolvedFields {
      ...FieldContent
    }
  }
`);

export const InterpFileType = graphql(/* GraphQL */ `
  fragment InterpFile on File {
    id
    revision
    name
    path
    parent {
      id
    }
    createdAt
    updatedAt
    deletedAt
  }
`);

export const InterpStatementType = graphql(/* GraphQL */ `
  fragment InterpStatement on Statement {
    id
    type
    symbolType
    name
    modifier
    revision
    createdAt
    updatedAt
    deletedAt
    file {
      id
    }
    parent {
      id
    }
    orderKey
    reference {
      id
    }
    referenceProjectVersion {
      id
    }
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
  }
`);
