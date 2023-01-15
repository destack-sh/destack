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

export const ProjectVersionAsDependencyType = graphql(/* GraphQL */ `
  fragment ProjectVersionAsDependency on ProjectVersion {
    id
    createdAt
    committedAt
    name
    files {
      ...FileHeader
    }
    project {
      id
      name
      slug
      path
      organization {
        id
        name
        slug
      }
    }
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    type
    name
    path
    pathWithoutExtension
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
    symbolType
    createdAt
    updatedAt
    deletedAt
    modifier
    name
    compiled
    commented
    index
    file {
      id
      path
      pathWithoutExtension
      projectVersion {
        id
      }
    }
    parent {
      id
    }
    reference {
      id
    }
  }
`);

export const TypeNodeContentDeepType = graphql(/* GraphQL */ `
  fragment TypeNodeContentDeep on TypeNode {
    name
    type
    required
    children {
      name
      type
      required
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
      ...StatementHeader
    }
    text
    # symbol contents
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
