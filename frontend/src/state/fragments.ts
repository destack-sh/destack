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
    createdAt
    updatedAt
    head {
      ...ProjectVersionHeader
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

export const SchemaElementContentDeepType = graphql(/* GraphQL */ `
  fragment SchemaElementContentDeep on SchemaElement {
    name
    type
    required
    schemaId
    choices
    elements {
      name
      type
      required
      schemaId
      choices
      elements {
        name
        type
        required
        schemaId
        choices
      }
    }
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
    content {
      ... on Code {
        ...CodeContent
      }
      ... on Dataset {
        ...DatasetContent
      }
      ... on Expectation {
        ...ExpectationContent
      }
      ... on Task {
        ...TaskContent
      }
      ... on Schema {
        ...SchemaContent
      }
    }
    reference {
      ...StatementHeader
    }
    dependency {
      projectVersion {
        ...ProjectVersionAsDependency
      }
    }
    text
  }
`);
