/* eslint-disable */
import * as types from "./graphql";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel-plugin for production.
 */
const documents = {
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          id\n          ...FileHeader\n        }\n      }\n    }\n  ":
    types.ProjectVersionContentDocument,
  "\n    query ownerBySlug($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n        }\n        ... on User {\n          id\n        }\n      }\n    }\n  ":
    types.OwnerBySlugDocument,
  "\n    query existingProjectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        id\n        slug\n      }\n    }\n  ":
    types.ExistingProjectBySlugDocument,
  "\n    query home {\n      me {\n        slug\n        projects {\n          id\n          name\n          slug\n          path\n          createdAt\n          type\n          visibility\n          createdAt\n        }\n        organizations {\n          projects {\n            id\n            name\n            path\n            slug\n            createdAt\n            type\n            visibility\n            createdAt\n          }\n        }\n      }\n    }\n  ":
    types.HomeDocument,
  "\n      query me {\n        me {\n          ...UserContent\n        }\n      }\n    ": types.MeDocument,
  "\n      query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n        project(id: $projectId) {\n          versions(filters: { afterId: $afterId }) {\n            id\n            name\n            createdAt\n            parentsRefs {\n              source\n              target\n            }\n          }\n        }\n      }\n    ":
    types.ProjectMigrationRefsDocument,
  "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    # note: do not query for non-id fields on root/parent here since\n    # they may not be available when streamed directly from the runtime\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n    }\n    task {\n      id\n    }\n    code {\n      id\n    }\n    model {\n      id\n    }\n  }\n":
    types.ExecutionContentFragmentDoc,
  "\n      query executions(\n        $projectVersionId: GlobalID!\n        $buildId: GlobalID\n        $taskId: GlobalID\n        $codeId: GlobalID\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectVersionId: $projectVersionId\n          buildId: $buildId\n          taskId: $taskId\n          codeId: $codeId\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    ":
    types.ExecutionsDocument,
  "\n        subscription moduleExecutionChanged(\n          $projectVersionId: GlobalID!\n          $buildId: GlobalID\n          $taskId: GlobalID\n          $codeId: GlobalID\n          $rootIdNull: Boolean\n        ) {\n          moduleExecutionChanged(\n            projectVersionId: $projectVersionId\n            buildId: $buildId\n            taskId: $taskId\n            codeId: $codeId\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      ":
    types.ModuleExecutionChangedDocument,
  "\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n":
    types.PageInfoFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment UserContent on User {\n    id\n    username\n    slug\n    email\n    firstName\n    createdAt\n    updatedAt\n    completedSignup\n    organizations {\n      id\n      name\n      slug\n      createdAt\n      updatedAt\n    }\n  }\n":
    types.UserContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        firstName\n      }\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    generated\n    projectVersion {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment TypeContent on Type {\n    description\n  }\n": types.TypeContentFragmentDoc,
  "\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n":
    types.SimpleTypeNodeContentFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    rootTypeTag\n    typeNodes {\n      ...SimpleTypeNodeContent\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            ...FileHeader\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!, $path: String!) {\n        renameFile(input: { id: $id, name: $name, path: $path }) {\n          ... on File {\n            id\n            name\n            path\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation createProject($input: ProjectCreateInput!) {\n        createProject(input: $input) {\n          ... on Project {\n            ...ProjectHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateProjectDocument,
  "\n      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {\n        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {\n          ... on Project {\n            id\n            visibility\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectVisibilityDocument,
  "\n      mutation updateProjectName($id: GlobalID!, $name: String!) {\n        updateProjectName(input: { id: $id, name: $name }) {\n          ... on Project {\n            id\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectNameDocument,
  "\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n            buildIds\n          }\n        }\n      }\n    ":
    types.BuildDocument,
  "\n      mutation run($projectVersionId: GlobalID!, $runnableId: GlobalID, $buildId: GlobalID, $arguments: JSON!) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n          }\n        }\n      }\n    ":
    types.RunDocument,
  "\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            rootTypeTag\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
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
  "\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateTypeNodeDocument,
  "\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteTypeNodeDocument,
  "\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateTypeNodeDocument,
  "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementDescriptionDocument,
  "\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementCodeDocument,
  "\n      mutation updateStatementText($id: GlobalID!, $code: String) {\n        updateStatementText(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementTextDocument,
  "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateRecordDocument,
  "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateRecordDocument,
  "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteRecordDocument,
  "\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.LogoutDocument,
  "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            ...UserContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CompleteSignupDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommitDocument,
  "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n":
    types.InterpSymbolContentFragmentDoc,
  "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n":
    types.InterpModuleContentFragmentDoc,
  "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n":
    types.InterpErrorContentFragmentDoc,
  "\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          updatedAt\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    ":
    types.ModuleRuntimeChangedDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = gql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        versions {\n          ...ProjectVersionHeader\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "
): typeof documents["\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          id\n          ...FileHeader\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          id\n          ...FileHeader\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query ownerBySlug($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n        }\n        ... on User {\n          id\n        }\n      }\n    }\n  "
): typeof documents["\n    query ownerBySlug($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n        }\n        ... on User {\n          id\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query existingProjectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        id\n        slug\n      }\n    }\n  "
): typeof documents["\n    query existingProjectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        id\n        slug\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query home {\n      me {\n        slug\n        projects {\n          id\n          name\n          slug\n          path\n          createdAt\n          type\n          visibility\n          createdAt\n        }\n        organizations {\n          projects {\n            id\n            name\n            path\n            slug\n            createdAt\n            type\n            visibility\n            createdAt\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query home {\n      me {\n        slug\n        projects {\n          id\n          name\n          slug\n          path\n          createdAt\n          type\n          visibility\n          createdAt\n        }\n        organizations {\n          projects {\n            id\n            name\n            path\n            slug\n            createdAt\n            type\n            visibility\n            createdAt\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query me {\n        me {\n          ...UserContent\n        }\n      }\n    "
): typeof documents["\n      query me {\n        me {\n          ...UserContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n        project(id: $projectId) {\n          versions(filters: { afterId: $afterId }) {\n            id\n            name\n            createdAt\n            parentsRefs {\n              source\n              target\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {\n        project(id: $projectId) {\n          versions(filters: { afterId: $afterId }) {\n            id\n            name\n            createdAt\n            parentsRefs {\n              source\n              target\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    # note: do not query for non-id fields on root/parent here since\n    # they may not be available when streamed directly from the runtime\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n    }\n    task {\n      id\n    }\n    code {\n      id\n    }\n    model {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    # note: do not query for non-id fields on root/parent here since\n    # they may not be available when streamed directly from the runtime\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n    }\n    task {\n      id\n    }\n    code {\n      id\n    }\n    model {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query executions(\n        $projectVersionId: GlobalID!\n        $buildId: GlobalID\n        $taskId: GlobalID\n        $codeId: GlobalID\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectVersionId: $projectVersionId\n          buildId: $buildId\n          taskId: $taskId\n          codeId: $codeId\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "
): typeof documents["\n      query executions(\n        $projectVersionId: GlobalID!\n        $buildId: GlobalID\n        $taskId: GlobalID\n        $codeId: GlobalID\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectVersionId: $projectVersionId\n          buildId: $buildId\n          taskId: $taskId\n          codeId: $codeId\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription moduleExecutionChanged(\n          $projectVersionId: GlobalID!\n          $buildId: GlobalID\n          $taskId: GlobalID\n          $codeId: GlobalID\n          $rootIdNull: Boolean\n        ) {\n          moduleExecutionChanged(\n            projectVersionId: $projectVersionId\n            buildId: $buildId\n            taskId: $taskId\n            codeId: $codeId\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "
): typeof documents["\n        subscription moduleExecutionChanged(\n          $projectVersionId: GlobalID!\n          $buildId: GlobalID\n          $taskId: GlobalID\n          $codeId: GlobalID\n          $rootIdNull: Boolean\n        ) {\n          moduleExecutionChanged(\n            projectVersionId: $projectVersionId\n            buildId: $buildId\n            taskId: $taskId\n            codeId: $codeId\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n"
): typeof documents["\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"
): typeof documents["\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment UserContent on User {\n    id\n    username\n    slug\n    email\n    firstName\n    createdAt\n    updatedAt\n    completedSignup\n    organizations {\n      id\n      name\n      slug\n      createdAt\n      updatedAt\n    }\n  }\n"
): typeof documents["\n  fragment UserContent on User {\n    id\n    username\n    slug\n    email\n    firstName\n    createdAt\n    updatedAt\n    completedSignup\n    organizations {\n      id\n      name\n      slug\n      createdAt\n      updatedAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        firstName\n      }\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        firstName\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    generated\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    generated\n    projectVersion {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment TypeContent on Type {\n    description\n  }\n"
): typeof documents["\n  fragment TypeContent on Type {\n    description\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n"
): typeof documents["\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    rootTypeTag\n    typeNodes {\n      ...SimpleTypeNodeContent\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    value\n    rootTypeTag\n    typeNodes {\n      ...SimpleTypeNodeContent\n    }\n    records {\n      id\n      orderKey\n      data\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            ...FileHeader\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            ...FileHeader\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation renameFile($id: GlobalID!, $name: String!, $path: String!) {\n        renameFile(input: { id: $id, name: $name, path: $path }) {\n          ... on File {\n            id\n            name\n            path\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameFile($id: GlobalID!, $name: String!, $path: String!) {\n        renameFile(input: { id: $id, name: $name, path: $path }) {\n          ... on File {\n            id\n            name\n            path\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createProject($input: ProjectCreateInput!) {\n        createProject(input: $input) {\n          ... on Project {\n            ...ProjectHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createProject($input: ProjectCreateInput!) {\n        createProject(input: $input) {\n          ... on Project {\n            ...ProjectHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {\n        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {\n          ... on Project {\n            id\n            visibility\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {\n        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {\n          ... on Project {\n            id\n            visibility\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateProjectName($id: GlobalID!, $name: String!) {\n        updateProjectName(input: { id: $id, name: $name }) {\n          ... on Project {\n            id\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateProjectName($id: GlobalID!, $name: String!) {\n        updateProjectName(input: { id: $id, name: $name }) {\n          ... on Project {\n            id\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n            buildIds\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n            buildIds\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation run($projectVersionId: GlobalID!, $runnableId: GlobalID, $buildId: GlobalID, $arguments: JSON!) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation run($projectVersionId: GlobalID!, $runnableId: GlobalID, $buildId: GlobalID, $arguments: JSON!) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            rootTypeTag\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            rootTypeTag\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              ...StatementHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteTypeNode($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on Statement {\n            id\n            revision\n            typeNodes {\n              ...SimpleTypeNodeContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementText($id: GlobalID!, $code: String) {\n        updateStatementText(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementText($id: GlobalID!, $code: String) {\n        updateStatementText(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteStatementRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            records {\n              id\n              orderKey\n              data\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            ...UserContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            ...UserContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n"
): typeof documents["\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n"
): typeof documents["\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          updatedAt\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    "
): typeof documents["\n      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {\n        moduleRuntimeChanged(projectVersionId: $projectVersionId) {\n          updatedAt\n          module {\n            ...InterpModuleContent\n          }\n          dependencies {\n            ...InterpModuleContent\n          }\n          errors {\n            ...InterpErrorContent\n          }\n        }\n      }\n    "];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<
  infer TType,
  any
>
  ? TType
  : never;
