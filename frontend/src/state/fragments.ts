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

export const CrudModelType = graphql(/* GraphQL */ `
  fragment CrudModelContent on CrudModel {
    id
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    tag
    description
    committed
    committedAt
    parents {
      id
    }
    ...CrudModelContent
  }
`);

export const ProjectHeaderType = graphql(/* GraphQL */ `
  fragment ProjectHeader on Project {
    id
    type
    visibility
    createdAt
    updatedAt
    name
    slug
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
    parent {
      ... on File {
        id
      }
      ... on ProjectVersion {
        id
      }
    }
    directory
    projectVersion {
      id
    }
    deletedAt
    ...CrudModelContent
  }
`);

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    id
    type
    revision
    modifier
    name
    commented
    orderKey
    parent {
      ... on File {
        id
      }
      ... on Statement {
        id
      }
    }
    deletedAt
    ...CrudModelContent
  }
`);

export const FieldType = graphql(/* GraphQL */ `
  fragment FieldContent on Field {
    # :FieldContent
    id
    revision
    name
    key
    tag
    hint
    flags
    description
    orderKey
    reference {
      id
    }
    parent {
      id
    }
    metadata
    deletedAt
    ...CrudModelContent
  }
`);

export const StatementContentType = graphql(/* GraphQL */ `
  fragment StatementContent on Statement {
    id
    type
    revision
    name
    commented
    modifier
    orderKey
    parent {
      ... on Statement {
        id
      }
      ... on File {
        id
      }
    }
    # symbol contents
    text
    lang
    code
    description
    value
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
    deletedAt
    ...CrudModelContent
  }
`);

export const IssueContentType = graphql(/* GraphQL */ `
  fragment IssueContent on Issue {
    # :IssueContent
    id
    scope
    kind
    type
    message
    file {
      id
    }
    statement {
      id
    }
  }
`);

export const ResolvedFieldContentType = graphql(/* GraphQL */ `
  fragment ResolvedFieldContent on ResolvedField {
    id
    statement {
      id
    }
    field {
      ...FieldContent
    }
  }
`);

export const InterpFileType = graphql(/* GraphQL */ `
  fragment InterpFile on File {
    # :InterpFile
    id
    revision
    name
    directory
    parent {
      ... on File {
        id
      }
      ... on ProjectVersion {
        id
      }
    }
    issues {
      ...IssueContent
    }
    deletedAt
    ...CrudModelContent
  }
`);

export const InterpStatementType = graphql(/* GraphQL */ `
  fragment InterpStatement on Statement {
    # :InterpStatement
    id
    type
    name
    modifier
    revision
    file {
      id
    }
    parent {
      ... on Statement {
        id
      }
      ... on File {
        id
      }
    }
    orderKey
    referenceProjectVersion {
      id
    }
    rootTypeTag
    rootTypeFlags
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
    deletedAt
    ...CrudModelContent
  }
`);
