/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n  }\n":
    types.ExpectationContentFragmentDoc,
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n  fragment TaskContent on Task {\n    id\n    description\n  }\n": types.TaskContentFragmentDoc,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n":
    types.ProjectVersionContentFragmentDoc,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  ":
    types.ProjectVersionContentDocument,
  "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n  }\n": types.CodeContentFragmentDoc,
  "\n  fragment DatasetContent on Dataset {\n    id\n    length\n    records {\n      data\n      index\n    }\n  }\n":
    types.DatasetContentFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment DependencyHeader on ProjectVersion {\n    id\n    createdAt\n    committedAt\n    name\n    files {\n      ...FileHeader\n    }\n    project {\n      id\n      name\n      slug\n      path\n      organization {\n        id\n        name\n        slug\n      }\n    }\n  }\n":
    types.DependencyHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    type\n    name\n    path\n    pathWithoutExtension\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    file {\n      id\n      path\n      pathWithoutExtension\n      projectVersion {\n        id\n      }\n    }\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n":
    types.CompilationHeaderFragmentDoc,
  "\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    required\n    schemaId\n    choices\n    elements {\n      name\n      type\n      required\n      schemaId\n      choices\n      elements {\n        name\n        type\n        required\n        schemaId\n        choices\n      }\n    }\n  }\n":
    types.SchemaElementContentDeepFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    content {\n      ... on Code {\n        ...CodeContent\n      }\n      ... on Dataset {\n        ...DatasetContent\n      }\n      ... on Expectation {\n        ...ExpectationContent\n      }\n      ... on Task {\n        ...TaskContent\n      }\n      ... on Schema {\n        ...SchemaContent\n      }\n    }\n    reference {\n      ...StatementHeader\n    }\n    parameters {\n      name\n      type\n    }\n    arguments {\n      name\n      value\n      reference {\n        ...StatementHeader\n      }\n    }\n    text\n  }\n":
    types.StatementContentFragmentDoc,
  "\n  fragment ProjectVersionContentSense on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      ...FileHeader\n      # exact same query as fileContentById to get immediate updates\n      statements(filters: { isVisible: true }) {\n        ...StatementHeader\n      }\n    }\n    dependencies {\n      id\n      ...DependencyHeader\n    }\n  }\n":
    types.ProjectVersionContentSenseFragmentDoc,
  "\n      query projectVersionContentSense($id: GlobalID!) {\n        projectVersion(id: $id) {\n          id\n          ...ProjectVersionContentSense\n        }\n      }\n    ":
    types.ProjectVersionContentSenseDocument,
  "\n      query schemaContentById($fileId: GlobalID!, $statementId: GlobalID!) {\n        file(id: $fileId) {\n          statements(filters: { id: $statementId }) {\n            id\n            ...StatementContent\n          }\n        }\n      }\n    ":
    types.SchemaContentByIdDocument,
  "\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    ":
    types.AddCompilationDocument,
  "\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    ":
    types.CompileDocument,
  "\n      mutation createFile($projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { projectVersion: { id: $projectVersionId }, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation createStatement(\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(input: { fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            file {\n              id\n              path\n              statements {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { statementId: $id, type: $type, symbolType: $symbolType }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            content {\n              # not re-using symbol content fragment because that led to weird apollo errors\n              ... on Code {\n                ...CodeContent\n              }\n              ... on Dataset {\n                ...DatasetContent\n              }\n              ... on Expectation {\n                ...ExpectationContent\n              }\n              ... on Task {\n                ...TaskContent\n              }\n              ... on Schema {\n                ...SchemaContent\n              }\n            }\n          }\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation setModifierStatement($id: GlobalID!, $modifier: StatementModifier) {\n        setModifierStatement(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n          }\n        }\n      }\n    ":
    types.SetModifierStatementDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          statement {\n            id\n            index\n            file {\n              id\n              path\n            }\n            parent {\n              id\n            }\n          }\n          oldFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n          newFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameStatementDocument,
  "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    ":
    types.DeleteStatementDocument,
  "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    ":
    types.RestoreStatementDocument,
  "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          statement {\n            id\n            commented\n            descendants {\n              id\n              commented\n            }\n          }\n        }\n      }\n    ":
    types.CommentStatementDocument,
  "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        setReferenceStatement(input: { statementId: $id, referenceId: $referenceId }) {\n          statement {\n            id\n            reference {\n              ...StatementHeader\n            }\n          }\n        }\n      }\n    ":
    types.SetReferenceDocument,
  "\n      mutation updateTaskContent($id: GlobalID!, $description: String!) {\n        updateTaskContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Task {\n              description\n            }\n          }\n        }\n      }\n    ":
    types.UpdateTaskContentDocument,
  "\n      mutation updateExpectationContent($id: GlobalID!, $description: String!) {\n        updateExpectationContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Expectation {\n              description\n            }\n          }\n        }\n      }\n    ":
    types.UpdateExpectationContentDocument,
  "\n      mutation updateCodeContent($id: GlobalID!, $code: String, $builtinId: String) {\n        updateCodeContent(input: { statementId: $id, code: $code, builtinId: $builtinId }) {\n          id\n          content {\n            id\n            ... on Code {\n              builtinId\n              code\n            }\n          }\n        }\n      }\n    ":
    types.UpdateCodeContentDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    ":
    types.CommitDocument,
  "\n  fragment SchemaContent on Schema {\n    id\n    description\n    element {\n      ...SchemaElementContentDeep\n    }\n  }\n":
    types.SchemaContentFragmentDoc,
};

export function graphql(
  source: "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n  }\n"
): typeof documents["\n  fragment ExpectationContent on Expectation {\n    id\n    description\n  }\n"];
export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment TaskContent on Task {\n    id\n    description\n  }\n"
): typeof documents["\n  fragment TaskContent on Task {\n    id\n    description\n  }\n"];
export function graphql(
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "
): typeof documents["\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"];
export function graphql(
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n  }\n"
): typeof documents["\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n  }\n"];
export function graphql(
  source: "\n  fragment DatasetContent on Dataset {\n    id\n    length\n    records {\n      data\n      index\n    }\n  }\n"
): typeof documents["\n  fragment DatasetContent on Dataset {\n    id\n    length\n    records {\n      data\n      index\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"
): typeof documents["\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment DependencyHeader on ProjectVersion {\n    id\n    createdAt\n    committedAt\n    name\n    files {\n      ...FileHeader\n    }\n    project {\n      id\n      name\n      slug\n      path\n      organization {\n        id\n        name\n        slug\n      }\n    }\n  }\n"
): typeof documents["\n  fragment DependencyHeader on ProjectVersion {\n    id\n    createdAt\n    committedAt\n    name\n    files {\n      ...FileHeader\n    }\n    project {\n      id\n      name\n      slug\n      path\n      organization {\n        id\n        name\n        slug\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    type\n    name\n    path\n    pathWithoutExtension\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    type\n    name\n    path\n    pathWithoutExtension\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment StatementHeader on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    file {\n      id\n      path\n      pathWithoutExtension\n      projectVersion {\n        id\n      }\n    }\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    file {\n      id\n      path\n      pathWithoutExtension\n      projectVersion {\n        id\n      }\n    }\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"
): typeof documents["\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n  }\n"];
export function graphql(
  source: "\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    required\n    schemaId\n    choices\n    elements {\n      name\n      type\n      required\n      schemaId\n      choices\n      elements {\n        name\n        type\n        required\n        schemaId\n        choices\n      }\n    }\n  }\n"
): typeof documents["\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    required\n    schemaId\n    choices\n    elements {\n      name\n      type\n      required\n      schemaId\n      choices\n      elements {\n        name\n        type\n        required\n        schemaId\n        choices\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    content {\n      ... on Code {\n        ...CodeContent\n      }\n      ... on Dataset {\n        ...DatasetContent\n      }\n      ... on Expectation {\n        ...ExpectationContent\n      }\n      ... on Task {\n        ...TaskContent\n      }\n      ... on Schema {\n        ...SchemaContent\n      }\n    }\n    reference {\n      ...StatementHeader\n    }\n    parameters {\n      name\n      type\n    }\n    arguments {\n      name\n      value\n      reference {\n        ...StatementHeader\n      }\n    }\n    text\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    content {\n      ... on Code {\n        ...CodeContent\n      }\n      ... on Dataset {\n        ...DatasetContent\n      }\n      ... on Expectation {\n        ...ExpectationContent\n      }\n      ... on Task {\n        ...TaskContent\n      }\n      ... on Schema {\n        ...SchemaContent\n      }\n    }\n    reference {\n      ...StatementHeader\n    }\n    parameters {\n      name\n      type\n    }\n    arguments {\n      name\n      value\n      reference {\n        ...StatementHeader\n      }\n    }\n    text\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectVersionContentSense on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      ...FileHeader\n      # exact same query as fileContentById to get immediate updates\n      statements(filters: { isVisible: true }) {\n        ...StatementHeader\n      }\n    }\n    dependencies {\n      id\n      ...DependencyHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionContentSense on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      ...FileHeader\n      # exact same query as fileContentById to get immediate updates\n      statements(filters: { isVisible: true }) {\n        ...StatementHeader\n      }\n    }\n    dependencies {\n      id\n      ...DependencyHeader\n    }\n  }\n"];
export function graphql(
  source: "\n      query projectVersionContentSense($id: GlobalID!) {\n        projectVersion(id: $id) {\n          id\n          ...ProjectVersionContentSense\n        }\n      }\n    "
): typeof documents["\n      query projectVersionContentSense($id: GlobalID!) {\n        projectVersion(id: $id) {\n          id\n          ...ProjectVersionContentSense\n        }\n      }\n    "];
export function graphql(
  source: "\n      query schemaContentById($fileId: GlobalID!, $statementId: GlobalID!) {\n        file(id: $fileId) {\n          statements(filters: { id: $statementId }) {\n            id\n            ...StatementContent\n          }\n        }\n      }\n    "
): typeof documents["\n      query schemaContentById($fileId: GlobalID!, $statementId: GlobalID!) {\n        file(id: $fileId) {\n          statements(filters: { id: $statementId }) {\n            id\n            ...StatementContent\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation createFile($projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { projectVersion: { id: $projectVersionId }, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createFile($projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { projectVersion: { id: $projectVersionId }, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          id\n          ...FileHeader\n          statements {\n            ...StatementHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation createStatement(\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(input: { fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            file {\n              id\n              path\n              statements {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement(\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(input: { fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            file {\n              id\n              path\n              statements {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { statementId: $id, type: $type, symbolType: $symbolType }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            content {\n              # not re-using symbol content fragment because that led to weird apollo errors\n              ... on Code {\n                ...CodeContent\n              }\n              ... on Dataset {\n                ...DatasetContent\n              }\n              ... on Expectation {\n                ...ExpectationContent\n              }\n              ... on Task {\n                ...TaskContent\n              }\n              ... on Schema {\n                ...SchemaContent\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { statementId: $id, type: $type, symbolType: $symbolType }) {\n          statement {\n            id\n            ...StatementHeader\n            text\n            content {\n              # not re-using symbol content fragment because that led to weird apollo errors\n              ... on Code {\n                ...CodeContent\n              }\n              ... on Dataset {\n                ...DatasetContent\n              }\n              ... on Expectation {\n                ...ExpectationContent\n              }\n              ... on Task {\n                ...TaskContent\n              }\n              ... on Schema {\n                ...SchemaContent\n              }\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation setModifierStatement($id: GlobalID!, $modifier: StatementModifier) {\n        setModifierStatement(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation setModifierStatement($id: GlobalID!, $modifier: StatementModifier) {\n        setModifierStatement(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          statement {\n            id\n            index\n            file {\n              id\n              path\n            }\n            parent {\n              id\n            }\n          }\n          oldFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n          newFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          statement {\n            id\n            index\n            file {\n              id\n              path\n            }\n            parent {\n              id\n            }\n          }\n          oldFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n          newFile {\n            id\n            path\n            statements {\n              id\n              index\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          statement {\n            id\n            commented\n            descendants {\n              id\n              commented\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          statement {\n            id\n            commented\n            descendants {\n              id\n              commented\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        setReferenceStatement(input: { statementId: $id, referenceId: $referenceId }) {\n          statement {\n            id\n            reference {\n              ...StatementHeader\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        setReferenceStatement(input: { statementId: $id, referenceId: $referenceId }) {\n          statement {\n            id\n            reference {\n              ...StatementHeader\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateTaskContent($id: GlobalID!, $description: String!) {\n        updateTaskContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Task {\n              description\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation updateTaskContent($id: GlobalID!, $description: String!) {\n        updateTaskContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Task {\n              description\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateExpectationContent($id: GlobalID!, $description: String!) {\n        updateExpectationContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Expectation {\n              description\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation updateExpectationContent($id: GlobalID!, $description: String!) {\n        updateExpectationContent(input: { statementId: $id, description: $description }) {\n          id\n          content {\n            id\n            ... on Expectation {\n              description\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateCodeContent($id: GlobalID!, $code: String, $builtinId: String) {\n        updateCodeContent(input: { statementId: $id, code: $code, builtinId: $builtinId }) {\n          id\n          content {\n            id\n            ... on Code {\n              builtinId\n              code\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation updateCodeContent($id: GlobalID!, $code: String, $builtinId: String) {\n        updateCodeContent(input: { statementId: $id, code: $code, builtinId: $builtinId }) {\n          id\n          content {\n            id\n            ... on Code {\n              builtinId\n              code\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n  fragment SchemaContent on Schema {\n    id\n    description\n    element {\n      ...SchemaElementContentDeep\n    }\n  }\n"
): typeof documents["\n  fragment SchemaContent on Schema {\n    id\n    description\n    element {\n      ...SchemaElementContentDeep\n    }\n  }\n"];

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
