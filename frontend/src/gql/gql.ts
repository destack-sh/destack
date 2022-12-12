/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n":
    types.CodeContentFragmentDoc,
  "\n  fragment DatasetContent on Dataset {\n    id\n    schema {\n      ...SchemaElementContentDeep\n    }\n    length\n    records {\n      data\n      index\n    }\n  }\n":
    types.DatasetContentFragmentDoc,
  "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      name\n      typeNameDeclaration\n    }\n  }\n":
    types.ExpectationContentFragmentDoc,
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        symbols {\n          id\n          ...SymbolContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n  fragment CodeContentToRun on Code {\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n":
    types.CodeContentToRunFragmentDoc,
  "\n    query codeToRun($id: GlobalID!) {\n      symbol(id: $id) {\n        id\n        type\n        name\n        typeNameDeclaration\n        content {\n          id\n          ... on Code {\n            ...CodeContentToRun\n          }\n        }\n      }\n    }\n  ":
    types.CodeToRunDocument,
  "\n  fragment ExecutionHeader on Execution {\n    id\n    status\n    createdAt\n    startedAt\n    terminatedAt\n    durationMillis\n    code {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    model {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n":
    types.ExecutionHeaderFragmentDoc,
  "\n    mutation run($input: RunCodeInput!) {\n      run(input: $input) {\n        execution {\n          id\n          ...ExecutionHeader\n          children {\n            id\n            ...ExecutionHeader\n            children {\n              id\n              ...ExecutionHeader\n            }\n          }\n        }\n        outputs {\n          name\n          value\n        }\n      }\n    }\n  ":
    types.RunDocument,
  "\n  fragment TaskContent on Task {\n    id\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    expectations {\n      id\n      description\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n      statements {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    templateImplementation {\n      id\n      name\n      typeNameDeclaration\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n":
    types.TaskContentFragmentDoc,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    mainProgram {\n      id\n      name\n      type\n    }\n    files {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n":
    types.ProjectVersionContentFragmentDoc,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  ":
    types.ProjectVersionContentDocument,
  "\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    ":
    types.AddCompilationDocument,
  "\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    ":
    types.CompileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            path\n          }\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation renameSymbol($id: GlobalID!, $name: String!) {\n        renameSymbol(input: { id: $id, name: $name }) {\n          ... on Symbol {\n            id\n            name\n            typeNameDeclaration\n          }\n        }\n      }\n    ":
    types.RenameSymbolDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    ":
    types.CommitDocument,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n    task {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    backends {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetTask {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetCode {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n":
    types.CompilationHeaderFragmentDoc,
  "\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    choices\n    elements {\n      name\n      type\n      choices\n    }\n  }\n":
    types.SchemaElementContentDeepFragmentDoc,
  "\n  fragment SymbolContent on Symbol {\n    id\n    name\n    type\n    typeShortname\n    typeNameDeclaration\n    createdAt\n    updatedAt\n    generated\n    content {\n      ...CodeContent\n      ...DatasetContent\n      ...ExpectationContent\n      ...TaskContent\n    }\n  }\n":
    types.SymbolContentFragmentDoc,
};

export function graphql(
  source: "\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  fragment CodeContent on Code {\n    id\n    builtinId\n    code\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment DatasetContent on Dataset {\n    id\n    schema {\n      ...SchemaElementContentDeep\n    }\n    length\n    records {\n      data\n      index\n    }\n  }\n"
): typeof documents["\n  fragment DatasetContent on Dataset {\n    id\n    schema {\n      ...SchemaElementContentDeep\n    }\n    length\n    records {\n      data\n      index\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      name\n      typeNameDeclaration\n    }\n  }\n"
): typeof documents["\n  fragment ExpectationContent on Expectation {\n    id\n    description\n    statements {\n      id\n      name\n      typeNameDeclaration\n    }\n  }\n"];
export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        symbols {\n          id\n          ...SymbolContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        symbols {\n          id\n          ...SymbolContent\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment CodeContentToRun on Code {\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  fragment CodeContentToRun on Code {\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    symbol {\n      parameters {\n        name\n        type\n        schema {\n          ...SchemaElementContentDeep\n        }\n      }\n      arguments {\n        name\n        type\n        value\n        reference {\n          id\n          name\n          typeNameDeclaration\n        }\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n    query codeToRun($id: GlobalID!) {\n      symbol(id: $id) {\n        id\n        type\n        name\n        typeNameDeclaration\n        content {\n          id\n          ... on Code {\n            ...CodeContentToRun\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query codeToRun($id: GlobalID!) {\n      symbol(id: $id) {\n        id\n        type\n        name\n        typeNameDeclaration\n        content {\n          id\n          ... on Code {\n            ...CodeContentToRun\n          }\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment ExecutionHeader on Execution {\n    id\n    status\n    createdAt\n    startedAt\n    terminatedAt\n    durationMillis\n    code {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    model {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n"
): typeof documents["\n  fragment ExecutionHeader on Execution {\n    id\n    status\n    createdAt\n    startedAt\n    terminatedAt\n    durationMillis\n    code {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    model {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n    mutation run($input: RunCodeInput!) {\n      run(input: $input) {\n        execution {\n          id\n          ...ExecutionHeader\n          children {\n            id\n            ...ExecutionHeader\n            children {\n              id\n              ...ExecutionHeader\n            }\n          }\n        }\n        outputs {\n          name\n          value\n        }\n      }\n    }\n  "
): typeof documents["\n    mutation run($input: RunCodeInput!) {\n      run(input: $input) {\n        execution {\n          id\n          ...ExecutionHeader\n          children {\n            id\n            ...ExecutionHeader\n            children {\n              id\n              ...ExecutionHeader\n            }\n          }\n        }\n        outputs {\n          name\n          value\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment TaskContent on Task {\n    id\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    expectations {\n      id\n      description\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n      statements {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    templateImplementation {\n      id\n      name\n      typeNameDeclaration\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"
): typeof documents["\n  fragment TaskContent on Task {\n    id\n    inputSchema {\n      ...SchemaElementContentDeep\n    }\n    outputSchema {\n      ...SchemaElementContentDeep\n    }\n    expectations {\n      id\n      description\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n      statements {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    templateImplementation {\n      id\n      name\n      typeNameDeclaration\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"];
export function graphql(
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "
): typeof documents["\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    mainProgram {\n      id\n      name\n      type\n    }\n    files {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    mainProgram {\n      id\n      name\n      type\n    }\n    files {\n      id\n      ...FileHeader\n    }\n    compilations {\n      id\n      ...CompilationHeader\n    }\n  }\n"];
export function graphql(
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "];
export function graphql(
  source: "\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation addCompilation($input: AddCompilationInput!) {\n        addCompilationTarget(input: $input) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation compile($compilationId: GlobalID!) {\n        compile(input: { compilationId: $compilationId }) {\n          compilation {\n            id\n            name\n            createdAt\n            updatedAt\n            targetTask {\n              ...TaskContent\n            }\n            targetCode {\n              ...CodeContent\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            path\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            path\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameSymbol($id: GlobalID!, $name: String!) {\n        renameSymbol(input: { id: $id, name: $name }) {\n          ... on Symbol {\n            id\n            name\n            typeNameDeclaration\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation renameSymbol($id: GlobalID!, $name: String!) {\n        renameSymbol(input: { id: $id, name: $name }) {\n          ... on Symbol {\n            id\n            name\n            typeNameDeclaration\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    name\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    name\n    path\n    createdAt\n    updatedAt\n  }\n"];
export function graphql(
  source: "\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n    task {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    backends {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetTask {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetCode {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n"
): typeof documents["\n  fragment CompilationHeader on Compilation {\n    id\n    name\n    createdAt\n    updatedAt\n    task {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    backends {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetTask {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n    targetCode {\n      id\n      symbol {\n        id\n        name\n        typeNameDeclaration\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    choices\n    elements {\n      name\n      type\n      choices\n    }\n  }\n"
): typeof documents["\n  fragment SchemaElementContentDeep on SchemaElement {\n    name\n    type\n    choices\n    elements {\n      name\n      type\n      choices\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment SymbolContent on Symbol {\n    id\n    name\n    type\n    typeShortname\n    typeNameDeclaration\n    createdAt\n    updatedAt\n    generated\n    content {\n      ...CodeContent\n      ...DatasetContent\n      ...ExpectationContent\n      ...TaskContent\n    }\n  }\n"
): typeof documents["\n  fragment SymbolContent on Symbol {\n    id\n    name\n    type\n    typeShortname\n    typeNameDeclaration\n    createdAt\n    updatedAt\n    generated\n    content {\n      ...CodeContent\n      ...DatasetContent\n      ...ExpectationContent\n      ...TaskContent\n    }\n  }\n"];

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
