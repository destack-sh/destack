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
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment TypeContent on Type {\n    description\n    btl\n  }\n": types.TypeContentFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    code\n    codeBuiltinId\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    btl\n    records {\n      data\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n      mutation createFile($id: GlobalID, $projectVersionId: GlobalID!, $name: String!) {\n        createFile(input: { id: $id, projectVersionId: $projectVersionId, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            ...FileHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            ...FileHeader\n            statements {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(\n          input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }\n        ) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { id: $id, type: $type, symbolType: $symbolType }) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            code\n            codeBuiltinId\n            description\n            btl\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementModifierDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          ... on Statement {\n            id\n            index\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameStatementDocument,
  "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteStatementDocument,
  "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreStatementDocument,
  "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommentStatementDocument,
  "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SetReferenceDocument,
  "\n      mutation updateStatementTypeNode($id: GlobalID!, $btl: String!) {\n        updateStatementTypeNode(input: { id: $id, btl: $btl }) {\n          ... on Statement {\n            id\n            btl\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementTypeNodeDocument,
  "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementDescriptionDocument,
  "\n      mutation updateStatementCode($id: GlobalID!, $code: String, $codeBuiltinId: String) {\n        updateStatementCode(input: { id: $id, code: $code, codeBuiltinId: $codeBuiltinId }) {\n          ... on Statement {\n            id\n            code\n            codeBuiltinId\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementCodeDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    ":
    types.CommitDocument,
  "\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    type\n    description\n    required\n    reference\n  }\n":
    types.TypeNodeContentInnerFragmentDoc,
  "\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n":
    types.TypeNodeContentFragmentDoc,
  "\n  fragment InterpStatementContent on InterpStatement {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n":
    types.InterpStatementContentFragmentDoc,
  "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      statements {\n        ...InterpStatementContent\n      }\n    }\n  }\n":
    types.InterpModuleContentFragmentDoc,
  "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    statement {\n      ...InterpStatementContent\n    }\n  }\n":
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
  source: "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    compiled\n    commented\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment TypeContent on Type {\n    description\n    btl\n  }\n"
): typeof documents["\n  fragment TypeContent on Type {\n    description\n    btl\n  }\n"];
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    code\n    codeBuiltinId\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    btl\n    records {\n      data\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    compiled\n    modifier\n    index\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    importPath\n    text\n    # symbol contents\n    code\n    codeBuiltinId\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    btl\n    records {\n      data\n    }\n  }\n"];
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
  source: "\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(\n          input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }\n        ) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $index: Int\n        $type: StatementType!\n        $name: String\n      ) {\n        createStatement(\n          input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index, type: $type, name: $name }\n        ) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { id: $id, type: $type, symbolType: $symbolType }) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            code\n            codeBuiltinId\n            description\n            btl\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement($id: GlobalID!, $type: StatementType!, $symbolType: SymbolType) {\n        morphStatement(input: { id: $id, type: $type, symbolType: $symbolType }) {\n          ... on Statement {\n            id\n            ...StatementHeader\n            text\n            code\n            codeBuiltinId\n            description\n            btl\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          ... on Statement {\n            id\n            index\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {\n          ... on Statement {\n            id\n            index\n            revision\n            file {\n              id\n              path\n              # should match FileInterface query\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n            referencedBy {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            revision\n            descendants {\n              id\n              deletedAt\n            }\n            # update all indices of statements in the same file\n            file {\n              id\n              statements(filters: { isVisible: true }) {\n                id\n                index\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementTypeNode($id: GlobalID!, $btl: String!) {\n        updateStatementTypeNode(input: { id: $id, btl: $btl }) {\n          ... on Statement {\n            id\n            btl\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementTypeNode($id: GlobalID!, $btl: String!) {\n        updateStatementTypeNode(input: { id: $id, btl: $btl }) {\n          ... on Statement {\n            id\n            btl\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation updateStatementCode($id: GlobalID!, $code: String, $codeBuiltinId: String) {\n        updateStatementCode(input: { id: $id, code: $code, codeBuiltinId: $codeBuiltinId }) {\n          ... on Statement {\n            id\n            code\n            codeBuiltinId\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementCode($id: GlobalID!, $code: String, $codeBuiltinId: String) {\n        updateStatementCode(input: { id: $id, code: $code, codeBuiltinId: $codeBuiltinId }) {\n          ... on Statement {\n            id\n            code\n            codeBuiltinId\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
export function graphql(
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          project {\n            ...ProjectHeader\n          }\n          committedVersion {\n            ...ProjectVersionHeader\n          }\n          newWorkingVersion {\n            ...ProjectVersionHeader\n          }\n        }\n      }\n    "];
export function graphql(
  source: "\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    type\n    description\n    required\n    reference\n  }\n"
): typeof documents["\n  fragment TypeNodeContentInner on TypeNode {\n    name\n    type\n    description\n    required\n    reference\n  }\n"];
export function graphql(
  source: "\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  fragment TypeNodeContent on TypeNode {\n    ...TypeNodeContentInner\n    children {\n      ...TypeNodeContentInner\n      children {\n        ...TypeNodeContentInner\n        children {\n          ...TypeNodeContentInner\n        }\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpStatementContent on InterpStatement {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpStatementContent on InterpStatement {\n    id\n    name\n    type\n    modifier\n    symbolType\n    typeNode {\n      ...TypeNodeContent\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      statements {\n        ...InterpStatementContent\n      }\n    }\n  }\n"
): typeof documents["\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      statements {\n        ...InterpStatementContent\n      }\n    }\n  }\n"];
export function graphql(
  source: "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    statement {\n      ...InterpStatementContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    statement {\n      ...InterpStatementContent\n    }\n  }\n"];
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
