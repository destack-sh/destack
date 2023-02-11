/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

const documents = {
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  ":
    types.ProjectVersionContentDocument,
  "\n    query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n      project(id: $projectId) {\n        versions(filters: { afterId: $afterId }) {\n          id\n          name\n          createdAt\n          parentsRefs {\n            source\n            target\n          }\n        }\n      }\n    }\n  ":
    types.ProjectMigrationRefsDocument,
  "\n  fragment DatasetContent on Statement {\n    records {\n      data\n    }\n  }\n":
    types.DatasetContentFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    organization {\n      slug\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n  }\n":
    types.ProjectVersionContentFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment TypeContent on Type {\n    description\n  }\n": types.TypeContentFragmentDoc,
  "\n  fragment TypeNodeData on TypeNodeData {\n    id\n    name\n    tag\n    description\n    value\n    parentId\n    orderKey\n    reference\n  }\n":
    types.TypeNodeDataFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    typeNodes {\n      ...TypeNodeData\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n      mutation createFile($id: GlobalID, $projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { id: $id, projectVersionId: $projectVersionId, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation compile($projectVersionId: GlobalID!, $compilationId: GlobalID!) {\n        compile(input: { projectVersionId: $projectVersionId, compilationId: $compilationId }) {\n          ... on CompileState {\n            success\n          }\n        }\n      }\n    ":
    types.CompileDocument,
  "\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            typeNodes {\n              ...TypeNodeData\n            }\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementModifierDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameStatementDocument,
  "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteStatementDocument,
  "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreStatementDocument,
  "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommentStatementDocument,
  "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SetReferenceDocument,
  "\n      mutation updateTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    ":
    types.UpdateTypeNodeDocument,
  "\n      mutation createTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    ":
    types.CreateTypeNodeDocument,
  "\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    ":
    types.DeleteTypeNodeDocument,
  "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementDescriptionDocument,
  "\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementCodeDocument,
  "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateRecordDocument,
  "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateRecordDocument,
  "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteRecordDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    ":
    types.CommitDocument,
  "\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    tag\n    description\n    reference\n  }\n":
    types.TypeNodeContentInnerFragmentDoc,
  "\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n":
    types.TypeNodeContentFragmentDoc,
  "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n":
    types.InterpSymbolContentFragmentDoc,
  "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n":
    types.InterpModuleContentFragmentDoc,
  "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n":
    types.InterpErrorContentFragmentDoc,
  "\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    ":
    types.ModuleRuntimeChangedDocument,
};

export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "
): typeof documents["\n    query projectBySlug($organization: String!, $project: String!) {\n      projectBySlug(organization: $organization, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ...ProjectVersionContent\n      }\n    }\n  "];
export function graphql(
  source: "\n    query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n      project(id: $projectId) {\n        versions(filters: { afterId: $afterId }) {\n          id\n          name\n          createdAt\n          parentsRefs {\n            source\n            target\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n      project(id: $projectId) {\n        versions(filters: { afterId: $afterId }) {\n          id\n          name\n          createdAt\n          parentsRefs {\n            source\n            target\n          }\n        }\n      }\n    }\n  "];
export function graphql(
  source: "\n  fragment DatasetContent on Statement {\n    records {\n      data\n    }\n  }\n"
): typeof documents["\n  fragment DatasetContent on Statement {\n    records {\n      data\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"
): typeof documents["\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    organization {\n      slug\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    organization {\n      slug\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionContent on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    files(filters: { isVisible: true }) {\n      id\n      ...FileHeader\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    createdAt\n    updatedAt\n    deletedAt\n    projectVersion {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment TypeContent on Type {\n    description\n  }\n"
): typeof documents["\n  fragment TypeContent on Type {\n    description\n  }\n"];
export function graphql(
  source: "\n  fragment TypeNodeData on TypeNodeData {\n    id\n    name\n    tag\n    description\n    value\n    parentId\n    orderKey\n    reference\n  }\n"
): typeof documents["\n  fragment TypeNodeData on TypeNodeData {\n    id\n    name\n    tag\n    description\n    value\n    parentId\n    orderKey\n    reference\n  }\n"];
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    typeNodes {\n      ...TypeNodeData\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    typeNodes {\n      ...TypeNodeData\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n"];
export function graphql(
  source: "\n      mutation createFile($id: GlobalID, $projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { id: $id, projectVersionId: $projectVersionId, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createFile($id: GlobalID, $projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { id: $id, projectVersionId: $projectVersionId, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation compile($projectVersionId: GlobalID!, $compilationId: GlobalID!) {\n        compile(input: { projectVersionId: $projectVersionId, compilationId: $compilationId }) {\n          ... on CompileState {\n            success\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation compile($projectVersionId: GlobalID!, $compilationId: GlobalID!) {\n        compile(input: { projectVersionId: $projectVersionId, compilationId: $compilationId }) {\n          ... on CompileState {\n            success\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            typeNodes {\n              ...TypeNodeData\n            }\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            typeNodes {\n              ...TypeNodeData\n            }\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation updateTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation createTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation createTypeNode($typeNode: TypeNodeDataCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...TypeNodeData\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    tag\n    description\n    reference\n  }\n"
): typeof documents["\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    tag\n    description\n    reference\n  }\n"];
export function graphql(
  source: "\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n"
): typeof documents["\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n"];
export function graphql(
  source: "\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    "
): typeof documents["\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    "];

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
