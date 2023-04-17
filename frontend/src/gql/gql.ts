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
  "\n    query deployments($projectVersionId: GlobalID!) {\n      projectVersion(id: $projectVersionId) {\n        id\n        committed\n        tag\n        deployments(filters: { isOwned: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              createdAt\n              updatedAt\n              type\n              status\n              deployAllStatements\n            }\n          }\n        }\n      }\n    }\n  ":
    types.DeploymentsDocument,
  "\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true, isGenerated: false }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  ":
    types.EmptyEditorSuggestedFilesDocument,
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        projectVersion {\n          id\n        }\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {\n      projectVersionByTag(projectId: $projectId, tag: $tag) {\n        id\n        tag\n      }\n    }\n  ":
    types.ExistingProjectVersionTagDocument,
  "\n    query runInfo($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        path\n        name\n        slug\n      }\n      projectVersion(id: $projectVersionId) {\n        id\n        name\n        tag\n        committed\n        createdAt\n        committedAt\n      }\n    }\n  ":
    types.RunInfoDocument,
  "\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  ":
    types.MatchingUsersDocument,
  "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.NotificationsDocument,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n    query profileAccessTokens($slug: String!, $includeInactive: Boolean!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProfileAccessTokensDocument,
  "\n    mutation createAccessToken(\n      $ownerId: GlobalID!\n      $scopes: [AccessTokenScope!]!\n      $expiresAt: DateTime\n      $name: String\n    ) {\n      createAccessToken(input: { ownerId: $ownerId, scopes: $scopes, expiresAt: $expiresAt, name: $name }) {\n        ... on AccessTokenCreatePayload {\n          token\n          accessToken {\n            id\n            name\n            tokenKey\n            createdAt\n            updatedAt\n            expiresAt\n            revokedAt\n            status\n            scopes\n          }\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.CreateAccessTokenDocument,
  "\n    mutation revokeAccessToken($id: GlobalID!) {\n      revokeAccessToken(id: $id) {\n        ... on AccessToken {\n          id\n          revokedAt\n          status\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.RevokeAccessTokenDocument,
  "\n    query organizationMembers($slug: String!) {\n      organizationBySlug(organization: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.OrganizationMembersDocument,
  "\n    query profileSettings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          description\n          canWrite\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          canWrite\n        }\n      }\n    }\n  ":
    types.ProfileSettingsDocument,
  "\n    mutation updateOrganization($id: GlobalID!, $name: String!, $description: String!) {\n      updateOrganization(input: { id: $id, name: $name, description: $description }) {\n        ... on Organization {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.UpdateOrganizationDocument,
  "\n    mutation updateUser($id: GlobalID!, $name: String!, $description: String!) {\n      updateUser(input: { id: $id, name: $name, description: $description }) {\n        ... on User {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.UpdateUserDocument,
  "\n      query checkOwnerBySlug($slug: String!) {\n        ownerBySlug(slug: $slug) {\n          ... on Organization {\n            id\n          }\n          ... on User {\n            id\n          }\n        }\n      }\n    ":
    types.CheckOwnerBySlugDocument,
  "\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              ...FileHeader\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProjectVersionContentDocument,
  "\n    query existingProjectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        id\n        slug\n      }\n    }\n  ":
    types.ExistingProjectBySlugDocument,
  "\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              type\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    type\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.HomeBenchesDocument,
  "\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            type\n            visibility\n            description\n          }\n        }\n      }\n    }\n  ":
    types.FeaturedBenchesDocument,
  "\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProfileHomeDocument,
  "\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          members {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  ":
    types.SettingsDocument,
  "\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          completedSignup\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.MeDocument,
  "\n      query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n        project(id: $projectId) {\n          migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n            isReverse\n            sourceVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            targetVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            refMappings {\n              type\n              sourceId\n              sourceVersionId\n              targetId\n              targetVersionId\n            }\n          }\n        }\n      }\n    ":
    types.ProjectMigrationRefsDocument,
  "\n  fragment EvaluationResultContent on EvaluationResult {\n    id\n    createdAt\n    updatedAt\n    kind\n    scope\n    project {\n      id\n    }\n    projectVersion {\n      id\n    }\n    build {\n      id\n    }\n    statement {\n      id\n    }\n    record {\n      id\n    }\n    typeNode {\n      id\n    }\n    selfMetrics\n    aggregatedMetrics\n  }\n":
    types.EvaluationResultContentFragmentDoc,
  "\n      query evaluations(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $includeAncestorVersions: Boolean\n        $scopeIn: [EvaluationScope!]\n        $kindIn: [EvaluationKind!]\n        $buildIdIn: [GlobalID!]\n        $systemIdIn: [GlobalID!]\n        $first: Int\n      ) {\n        evaluations(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          scopeIn: $scopeIn\n          kindIn: $kindIn\n          buildIdIn: $buildIdIn\n          systemIdIn: $systemIdIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...EvaluationResultContent\n            }\n          }\n        }\n      }\n    ":
    types.EvaluationsDocument,
  "\n        subscription evaluationsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $includeAncestorVersions: Boolean\n          $latestCandidateOnly: Boolean\n          $scopeIn: [EvaluationScope!]\n          $kindIn: [EvaluationKind!]\n          $buildIdIn: [GlobalID!]\n          $systemIdIn: [GlobalID!]\n        ) {\n          evaluationsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            latestCandidateOnly: $latestCandidateOnly\n            scopeIn: $scopeIn\n            kindIn: $kindIn\n            buildIdIn: $buildIdIn\n            systemIdIn: $systemIdIn\n          ) {\n            ...EvaluationResultContent\n          }\n        }\n      ":
    types.EvaluationsChangedDocument,
  "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    deployment {\n      id\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n      name\n    }\n    task {\n      id\n      name\n    }\n    code {\n      id\n      name\n    }\n  }\n":
    types.ExecutionContentFragmentDoc,
  "\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $buildIds: [GlobalID!]\n        $taskIds: [GlobalID!]\n        $codeIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          buildIds: $buildIds\n          taskIds: $taskIds\n          codeIds: $codeIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    ":
    types.ExecutionsDocument,
  "\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $buildIds: [GlobalID!]\n          $taskIds: [GlobalID!]\n          $codeIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            buildIds: $buildIds\n            taskIds: $taskIds\n            codeIds: $codeIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      ":
    types.ExecutionsChangedDocument,
  "\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n":
    types.PageInfoFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    tag\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    canWrite\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    generated\n    projectVersion {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    generated\n    commented\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment TypeContent on Type {\n    description\n  }\n": types.TypeContentFragmentDoc,
  "\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n":
    types.SimpleTypeNodeContentFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    typeNodes(filters: { isVisible: true }) {\n      ...SimpleTypeNodeContent\n    }\n    records(filters: { isVisible: true }) {\n      totalCount\n      edges {\n        node {\n          id\n          createdAt\n          updatedAt\n          deletedAt\n          revision\n          orderKey\n          data\n        }\n      }\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n  fragment JobContent on Job {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    type\n    projectVersion {\n      id\n    }\n  }\n":
    types.JobContentFragmentDoc,
  "\n      query jobs(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $statusIn: [JobStatus!]\n        $typeIn: [JobType!]\n        $first: Int\n      ) {\n        jobs(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          statusIn: $statusIn\n          typeIn: $typeIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...JobContent\n            }\n          }\n        }\n      }\n    ":
    types.JobsDocument,
  "\n        subscription jobsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!, $typeIn: [JobType!]) {\n          jobsChanged(projectId: $projectId, projectVersionId: $projectVersionId, typeIn: $typeIn) {\n            ...JobContent\n          }\n        }\n      ":
    types.JobsChangedDocument,
  "\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                invite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.NewNotificationsDocument,
  "\n      mutation markNotification($id: GlobalID!, $status: NotificationStatus!) {\n        markNotification(input: { id: $id, status: $status }) {\n          ... on Notification {\n            id\n            status\n            readAt\n            archivedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MarkNotificationDocument,
  "\n      mutation updateDeployment($id: GlobalID!, $status: DeploymentStatus!) {\n        updateDeployment(input: { id: $id, status: $status }) {\n          ... on Deployment {\n            id\n            type\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateDeploymentDocument,
  "\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            ...FileHeader\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!, $path: String!) {\n        renameFile(input: { id: $id, name: $name, path: $path }) {\n          ... on File {\n            id\n            name\n            path\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation createOrganization($name: String!, $slug: String!) {\n        createOrganization(input: { name: $name, slug: $slug }) {\n          ... on Organization {\n            id\n            name\n            slug\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateOrganizationDocument,
  "\n      mutation createInvites(\n        $id: GlobalID!\n        $emails: [String!]!\n        $level: OrganizationMembershipLevel!\n        $message: String\n      ) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateInvitesDocument,
  "\n      mutation cancelInvite($id: GlobalID!) {\n        cancelOrganizationInvite(id: $id) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CancelInviteDocument,
  "\n      mutation createProject($input: ProjectCreateInput!) {\n        createProject(input: $input) {\n          ... on Project {\n            ...ProjectHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateProjectDocument,
  "\n      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {\n        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {\n          ... on Project {\n            id\n            visibility\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectVisibilityDocument,
  "\n      mutation updateProjectName($id: GlobalID!, $name: String!) {\n        updateProjectName(input: { id: $id, name: $name }) {\n          ... on Project {\n            id\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectNameDocument,
  "\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n          }\n        }\n      }\n    ":
    types.BuildDocument,
  "\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $arguments: JSON!\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n            error\n            errorDetails\n          }\n        }\n      }\n    ":
    types.RunDocument,
  "\n      mutation createStatement($id: GlobalID, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        createStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            type\n            symbolType\n            revision\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            ...StatementContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement($input: StatementMorphInput!) {\n        morphStatement(input: $input) {\n          ... on Statement {\n            id\n            revision\n            type\n            symbolType\n            name\n            rootTypeTag\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation updateStatementModifier($id: GlobalID!, $modifier: StatementModifier) {\n        updateStatementModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementModifierDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchMoveStatementDocument,
  "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameStatementDocument,
  "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteStatementDocument,
  "\n      mutation batchDeleteStatements($ids: [GlobalID!]!) {\n        batchSoftDeleteStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchDeleteStatementsDocument,
  "\n      mutation restoreStatement($id: GlobalID!) {\n        restoreStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n            descendants {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreStatementDocument,
  "\n      mutation batchRestoreStatements($ids: [GlobalID!]!) {\n        batchRestoreStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchRestoreStatementsDocument,
  "\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchPasteStatementDocument,
  "\n      mutation commentStatement($id: GlobalID!, $commented: Boolean!) {\n        commentStatement(input: { id: $id, commented: $commented }) {\n          ... on Statement {\n            id\n            commented\n            revision\n            descendants {\n              id\n              commented\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommentStatementDocument,
  "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID, $referenceName: String) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId, referenceName: $referenceName }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SetReferenceDocument,
  "\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            statement {\n              id\n            }\n            ...SimpleTypeNodeContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateTypeNodeDocument,
  "\n      mutation deleteTypeNode($id: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteTypeNodeDocument,
  "\n      mutation restoreTypeNode($id: GlobalID!) {\n        restoreStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreTypeNodeDocument,
  "\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            updatedAt\n            revision\n            name\n            description\n            isOutput\n            isArray\n            isNullable\n            value\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateTypeNodeDocument,
  "\n      mutation updateStatementDescription($id: GlobalID!, $description: String!) {\n        updateStatementDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementDescriptionDocument,
  "\n      mutation updateStatementCode($id: GlobalID!, $code: String) {\n        updateStatementCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementCodeDocument,
  "\n      mutation updateStatementText($id: GlobalID!, $code: String) {\n        updateStatementText(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementTextDocument,
  "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on DatasetRecord {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statement {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateRecordDocument,
  "\n      mutation updateRecord($id: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, data: $data }) {\n          ... on DatasetRecord {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateRecordDocument,
  "\n      mutation deleteRecord($id: GlobalID!) {\n        deleteStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteRecordDocument,
  "\n      mutation restoreRecord($id: GlobalID!) {\n        restoreStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreRecordDocument,
  "\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.LogoutDocument,
  "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CompleteSignupDocument,
  "\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.AcceptOrganizationInviteDocument,
  "\n      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {\n        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {\n          ... on ProjectVersion {\n            ...ProjectVersionHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateVersionDocument,
  "\n      mutation commit(\n        $projectVersionId: GlobalID!\n        $name: String\n        $tag: String\n        $description: String\n        $autoDeploy: Boolean\n      ) {\n        commit(\n          input: {\n            projectVersionId: $projectVersionId\n            name: $name\n            tag: $tag\n            description: $description\n            autoDeploy: $autoDeploy\n          }\n        ) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommitDocument,
  "\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreDocument,
  "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    availableBuilds\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n":
    types.InterpSymbolContentFragmentDoc,
  "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n    dependencies {\n      id\n      name\n      files {\n        id\n        path\n        symbols {\n          ...InterpSymbolContent\n        }\n      }\n    }\n    errors {\n      ...InterpErrorContent\n    }\n    staleSymbols {\n      id\n      name\n      type\n      symbolType\n      modifier\n      parentId\n      rootTypeTag\n      generated\n    }\n  }\n":
    types.InterpModuleContentFragmentDoc,
  "\n  fragment InterpErrorContent on InterpError {\n    type\n    message\n    symbol {\n      ...InterpSymbolContent\n    }\n  }\n":
    types.InterpErrorContentFragmentDoc,
  "\n      subscription interpChanged($projectVersionId: GlobalID!) {\n        interpChanged(projectVersionId: $projectVersionId) {\n          ...InterpModuleContent\n        }\n      }\n    ":
    types.InterpChangedDocument,
  "\n      query systemInfo {\n        systemInfo {\n          version\n          gitCommit\n        }\n      }\n    ":
    types.SystemInfoDocument,
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
  source: "\n    query deployments($projectVersionId: GlobalID!) {\n      projectVersion(id: $projectVersionId) {\n        id\n        committed\n        tag\n        deployments(filters: { isOwned: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              createdAt\n              updatedAt\n              type\n              status\n              deployAllStatements\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query deployments($projectVersionId: GlobalID!) {\n      projectVersion(id: $projectVersionId) {\n        id\n        committed\n        tag\n        deployments(filters: { isOwned: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              createdAt\n              updatedAt\n              type\n              status\n              deployAllStatements\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true, isGenerated: false }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true, isGenerated: false }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        projectVersion {\n          id\n        }\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        projectVersion {\n          id\n        }\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {\n      projectVersionByTag(projectId: $projectId, tag: $tag) {\n        id\n        tag\n      }\n    }\n  "
): typeof documents["\n    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {\n      projectVersionByTag(projectId: $projectId, tag: $tag) {\n        id\n        tag\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query runInfo($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        path\n        name\n        slug\n      }\n      projectVersion(id: $projectVersionId) {\n        id\n        name\n        tag\n        committed\n        createdAt\n        committedAt\n      }\n    }\n  "
): typeof documents["\n    query runInfo($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        path\n        name\n        slug\n      }\n      projectVersion(id: $projectVersionId) {\n        id\n        name\n        tag\n        committed\n        createdAt\n        committedAt\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query profileAccessTokens($slug: String!, $includeInactive: Boolean!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query profileAccessTokens($slug: String!, $includeInactive: Boolean!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    mutation createAccessToken(\n      $ownerId: GlobalID!\n      $scopes: [AccessTokenScope!]!\n      $expiresAt: DateTime\n      $name: String\n    ) {\n      createAccessToken(input: { ownerId: $ownerId, scopes: $scopes, expiresAt: $expiresAt, name: $name }) {\n        ... on AccessTokenCreatePayload {\n          token\n          accessToken {\n            id\n            name\n            tokenKey\n            createdAt\n            updatedAt\n            expiresAt\n            revokedAt\n            status\n            scopes\n          }\n        }\n        ...OperationInfoContent\n      }\n    }\n  "
): typeof documents["\n    mutation createAccessToken(\n      $ownerId: GlobalID!\n      $scopes: [AccessTokenScope!]!\n      $expiresAt: DateTime\n      $name: String\n    ) {\n      createAccessToken(input: { ownerId: $ownerId, scopes: $scopes, expiresAt: $expiresAt, name: $name }) {\n        ... on AccessTokenCreatePayload {\n          token\n          accessToken {\n            id\n            name\n            tokenKey\n            createdAt\n            updatedAt\n            expiresAt\n            revokedAt\n            status\n            scopes\n          }\n        }\n        ...OperationInfoContent\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    mutation revokeAccessToken($id: GlobalID!) {\n      revokeAccessToken(id: $id) {\n        ... on AccessToken {\n          id\n          revokedAt\n          status\n        }\n        ...OperationInfoContent\n      }\n    }\n  "
): typeof documents["\n    mutation revokeAccessToken($id: GlobalID!) {\n      revokeAccessToken(id: $id) {\n        ... on AccessToken {\n          id\n          revokedAt\n          status\n        }\n        ...OperationInfoContent\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query organizationMembers($slug: String!) {\n      organizationBySlug(organization: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query organizationMembers($slug: String!) {\n      organizationBySlug(organization: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query profileSettings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          description\n          canWrite\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          canWrite\n        }\n      }\n    }\n  "
): typeof documents["\n    query profileSettings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          description\n          canWrite\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          canWrite\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    mutation updateOrganization($id: GlobalID!, $name: String!, $description: String!) {\n      updateOrganization(input: { id: $id, name: $name, description: $description }) {\n        ... on Organization {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  "
): typeof documents["\n    mutation updateOrganization($id: GlobalID!, $name: String!, $description: String!) {\n      updateOrganization(input: { id: $id, name: $name, description: $description }) {\n        ... on Organization {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    mutation updateUser($id: GlobalID!, $name: String!, $description: String!) {\n      updateUser(input: { id: $id, name: $name, description: $description }) {\n        ... on User {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  "
): typeof documents["\n    mutation updateUser($id: GlobalID!, $name: String!, $description: String!) {\n      updateUser(input: { id: $id, name: $name, description: $description }) {\n        ... on User {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query checkOwnerBySlug($slug: String!) {\n        ownerBySlug(slug: $slug) {\n          ... on Organization {\n            id\n          }\n          ... on User {\n            id\n          }\n        }\n      }\n    "
): typeof documents["\n      query checkOwnerBySlug($slug: String!) {\n        ownerBySlug(slug: $slug) {\n          ... on Organization {\n            id\n          }\n          ... on User {\n            id\n          }\n        }\n      }\n    "];
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
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              ...FileHeader\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        files(filters: { isVisible: true }) {\n          totalCount\n          edges {\n            node {\n              id\n              ...FileHeader\n            }\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              type\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    type\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              type\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    type\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            type\n            visibility\n            description\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            type\n            visibility\n            description\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewFull\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                type\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          members {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewFull\n          canWrite\n          members {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          completedSignup\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          completedSignup\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n        project(id: $projectId) {\n          migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n            isReverse\n            sourceVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            targetVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            refMappings {\n              type\n              sourceId\n              sourceVersionId\n              targetId\n              targetVersionId\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n        project(id: $projectId) {\n          migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n            isReverse\n            sourceVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            targetVersion {\n              id\n              createdAt\n              tag\n              name\n            }\n            refMappings {\n              type\n              sourceId\n              sourceVersionId\n              targetId\n              targetVersionId\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment EvaluationResultContent on EvaluationResult {\n    id\n    createdAt\n    updatedAt\n    kind\n    scope\n    project {\n      id\n    }\n    projectVersion {\n      id\n    }\n    build {\n      id\n    }\n    statement {\n      id\n    }\n    record {\n      id\n    }\n    typeNode {\n      id\n    }\n    selfMetrics\n    aggregatedMetrics\n  }\n"
): typeof documents["\n  fragment EvaluationResultContent on EvaluationResult {\n    id\n    createdAt\n    updatedAt\n    kind\n    scope\n    project {\n      id\n    }\n    projectVersion {\n      id\n    }\n    build {\n      id\n    }\n    statement {\n      id\n    }\n    record {\n      id\n    }\n    typeNode {\n      id\n    }\n    selfMetrics\n    aggregatedMetrics\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query evaluations(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $includeAncestorVersions: Boolean\n        $scopeIn: [EvaluationScope!]\n        $kindIn: [EvaluationKind!]\n        $buildIdIn: [GlobalID!]\n        $systemIdIn: [GlobalID!]\n        $first: Int\n      ) {\n        evaluations(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          scopeIn: $scopeIn\n          kindIn: $kindIn\n          buildIdIn: $buildIdIn\n          systemIdIn: $systemIdIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...EvaluationResultContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query evaluations(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $includeAncestorVersions: Boolean\n        $scopeIn: [EvaluationScope!]\n        $kindIn: [EvaluationKind!]\n        $buildIdIn: [GlobalID!]\n        $systemIdIn: [GlobalID!]\n        $first: Int\n      ) {\n        evaluations(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          scopeIn: $scopeIn\n          kindIn: $kindIn\n          buildIdIn: $buildIdIn\n          systemIdIn: $systemIdIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...EvaluationResultContent\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription evaluationsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $includeAncestorVersions: Boolean\n          $latestCandidateOnly: Boolean\n          $scopeIn: [EvaluationScope!]\n          $kindIn: [EvaluationKind!]\n          $buildIdIn: [GlobalID!]\n          $systemIdIn: [GlobalID!]\n        ) {\n          evaluationsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            latestCandidateOnly: $latestCandidateOnly\n            scopeIn: $scopeIn\n            kindIn: $kindIn\n            buildIdIn: $buildIdIn\n            systemIdIn: $systemIdIn\n          ) {\n            ...EvaluationResultContent\n          }\n        }\n      "
): typeof documents["\n        subscription evaluationsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $includeAncestorVersions: Boolean\n          $latestCandidateOnly: Boolean\n          $scopeIn: [EvaluationScope!]\n          $kindIn: [EvaluationKind!]\n          $buildIdIn: [GlobalID!]\n          $systemIdIn: [GlobalID!]\n        ) {\n          evaluationsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            latestCandidateOnly: $latestCandidateOnly\n            scopeIn: $scopeIn\n            kindIn: $kindIn\n            buildIdIn: $buildIdIn\n            systemIdIn: $systemIdIn\n          ) {\n            ...EvaluationResultContent\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    deployment {\n      id\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n      name\n    }\n    task {\n      id\n      name\n    }\n    code {\n      id\n      name\n    }\n  }\n"
): typeof documents["\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    deployment {\n      id\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    error\n    build {\n      id\n      name\n    }\n    task {\n      id\n      name\n    }\n    code {\n      id\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $buildIds: [GlobalID!]\n        $taskIds: [GlobalID!]\n        $codeIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          buildIds: $buildIds\n          taskIds: $taskIds\n          codeIds: $codeIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "
): typeof documents["\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $buildIds: [GlobalID!]\n        $taskIds: [GlobalID!]\n        $codeIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          buildIds: $buildIds\n          taskIds: $taskIds\n          codeIds: $codeIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $buildIds: [GlobalID!]\n          $taskIds: [GlobalID!]\n          $codeIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            buildIds: $buildIds\n            taskIds: $taskIds\n            codeIds: $codeIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "
): typeof documents["\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $buildIds: [GlobalID!]\n          $taskIds: [GlobalID!]\n          $codeIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            buildIds: $buildIds\n            taskIds: $taskIds\n            codeIds: $codeIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "];
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
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    tag\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    tag\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    canWrite\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    canWrite\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    generated\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    generated\n    projectVersion {\n      id\n    }\n  }\n"];
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
  source: "\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n"
): typeof documents["\n  fragment SimpleTypeNodeContent on SimpleTypeNode {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    tag\n    description\n    value\n    orderKey\n    reference {\n      id\n    }\n    isOutput\n    isArray\n    isNullable\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    typeNodes(filters: { isVisible: true }) {\n      ...SimpleTypeNodeContent\n    }\n    records(filters: { isVisible: true }) {\n      totalCount\n      edges {\n        node {\n          id\n          createdAt\n          updatedAt\n          deletedAt\n          revision\n          orderKey\n          data\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    symbolType\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    generated\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    reference {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    typeNodes(filters: { isVisible: true }) {\n      ...SimpleTypeNodeContent\n    }\n    records(filters: { isVisible: true }) {\n      totalCount\n      edges {\n        node {\n          id\n          createdAt\n          updatedAt\n          deletedAt\n          revision\n          orderKey\n          data\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment JobContent on Job {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    type\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment JobContent on Job {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    status\n    type\n    projectVersion {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query jobs(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $statusIn: [JobStatus!]\n        $typeIn: [JobType!]\n        $first: Int\n      ) {\n        jobs(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          statusIn: $statusIn\n          typeIn: $typeIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...JobContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query jobs(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID!\n        $statusIn: [JobStatus!]\n        $typeIn: [JobType!]\n        $first: Int\n      ) {\n        jobs(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          statusIn: $statusIn\n          typeIn: $typeIn\n          first: $first\n        ) {\n          totalCount\n          edges {\n            node {\n              ...JobContent\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription jobsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!, $typeIn: [JobType!]) {\n          jobsChanged(projectId: $projectId, projectVersionId: $projectVersionId, typeIn: $typeIn) {\n            ...JobContent\n          }\n        }\n      "
): typeof documents["\n        subscription jobsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!, $typeIn: [JobType!]) {\n          jobsChanged(projectId: $projectId, projectVersionId: $projectVersionId, typeIn: $typeIn) {\n            ...JobContent\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                invite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                invite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation markNotification($id: GlobalID!, $status: NotificationStatus!) {\n        markNotification(input: { id: $id, status: $status }) {\n          ... on Notification {\n            id\n            status\n            readAt\n            archivedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation markNotification($id: GlobalID!, $status: NotificationStatus!) {\n        markNotification(input: { id: $id, status: $status }) {\n          ... on Notification {\n            id\n            status\n            readAt\n            archivedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateDeployment($id: GlobalID!, $status: DeploymentStatus!) {\n        updateDeployment(input: { id: $id, status: $status }) {\n          ... on Deployment {\n            id\n            type\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateDeployment($id: GlobalID!, $status: DeploymentStatus!) {\n        updateDeployment(input: { id: $id, status: $status }) {\n          ... on Deployment {\n            id\n            type\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation createOrganization($name: String!, $slug: String!) {\n        createOrganization(input: { name: $name, slug: $slug }) {\n          ... on Organization {\n            id\n            name\n            slug\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createOrganization($name: String!, $slug: String!) {\n        createOrganization(input: { name: $name, slug: $slug }) {\n          ... on Organization {\n            id\n            name\n            slug\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createInvites(\n        $id: GlobalID!\n        $emails: [String!]!\n        $level: OrganizationMembershipLevel!\n        $message: String\n      ) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createInvites(\n        $id: GlobalID!\n        $emails: [String!]!\n        $level: OrganizationMembershipLevel!\n        $message: String\n      ) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation cancelInvite($id: GlobalID!) {\n        cancelOrganizationInvite(id: $id) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation cancelInvite($id: GlobalID!) {\n        cancelOrganizationInvite(id: $id) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {\n        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {\n          ... on BuildState {\n            projectVersionId\n            success\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $arguments: JSON!\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n            error\n            errorDetails\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $arguments: JSON!\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            buildId\n            output\n            success\n            error\n            errorDetails\n          }\n        }\n      }\n    "];
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
  source: "\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation batchDeleteStatements($ids: [GlobalID!]!) {\n        batchSoftDeleteStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchDeleteStatements($ids: [GlobalID!]!) {\n        batchSoftDeleteStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation batchRestoreStatements($ids: [GlobalID!]!) {\n        batchRestoreStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchRestoreStatements($ids: [GlobalID!]!) {\n        batchRestoreStatement(input: { ids: $ids }) {\n          ... on StatementBatch {\n            statements {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID, $referenceName: String) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId, referenceName: $referenceName }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation setReference($id: GlobalID!, $referenceId: GlobalID, $referenceName: String) {\n        updateStatementReference(input: { id: $id, referenceId: $referenceId, referenceName: $referenceName }) {\n          ... on Statement {\n            id\n            revision\n            reference {\n              id\n              name\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            statement {\n              id\n            }\n            ...SimpleTypeNodeContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createTypeNode($typeNode: TypeNodeCreateInput!) {\n        createStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            statement {\n              id\n            }\n            ...SimpleTypeNodeContent\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteTypeNode($id: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteTypeNode($id: GlobalID!) {\n        deleteStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreTypeNode($id: GlobalID!) {\n        restoreStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreTypeNode($id: GlobalID!) {\n        restoreStatementTypeNode(input: { id: $id }) {\n          ... on SimpleTypeNode {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            updatedAt\n            revision\n            name\n            description\n            isOutput\n            isArray\n            isNullable\n            value\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateTypeNode($typeNode: TypeNodeUpdateInput!) {\n        updateStatementTypeNode(input: $typeNode) {\n          ... on SimpleTypeNode {\n            id\n            updatedAt\n            revision\n            name\n            description\n            isOutput\n            isArray\n            isNullable\n            value\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on DatasetRecord {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statement {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createStatementRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on DatasetRecord {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statement {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateRecord($id: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, data: $data }) {\n          ... on DatasetRecord {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateRecord($id: GlobalID!, $data: JSON!) {\n        updateStatementRecord(input: { id: $id, data: $data }) {\n          ... on DatasetRecord {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteRecord($id: GlobalID!) {\n        deleteStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteRecord($id: GlobalID!) {\n        deleteStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreRecord($id: GlobalID!) {\n        restoreStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreRecord($id: GlobalID!) {\n        restoreStatementRecord(input: { id: $id }) {\n          ... on DatasetRecord {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {\n        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {\n          ... on ProjectVersion {\n            ...ProjectVersionHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {\n        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {\n          ... on ProjectVersion {\n            ...ProjectVersionHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation commit(\n        $projectVersionId: GlobalID!\n        $name: String\n        $tag: String\n        $description: String\n        $autoDeploy: Boolean\n      ) {\n        commit(\n          input: {\n            projectVersionId: $projectVersionId\n            name: $name\n            tag: $tag\n            description: $description\n            autoDeploy: $autoDeploy\n          }\n        ) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commit(\n        $projectVersionId: GlobalID!\n        $name: String\n        $tag: String\n        $description: String\n        $autoDeploy: Boolean\n      ) {\n        commit(\n          input: {\n            projectVersionId: $projectVersionId\n            name: $name\n            tag: $tag\n            description: $description\n            autoDeploy: $autoDeploy\n          }\n        ) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n            newWorkingVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    availableBuilds\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n"
): typeof documents["\n  fragment InterpSymbolContent on InterpSymbol {\n    id\n    name\n    type\n    orderKey\n    parentId\n    modifier\n    symbolType\n    rootTypeTag\n    generated\n    availableBuilds\n    typeNodes {\n      # not using SimpleTypeNodeContent fragment because it's for the editable node\n      # and using a shared fragment seems overkill\n      id\n      name\n      tag\n      description\n      value\n      orderKey\n      reference {\n        id\n      }\n      isOutput\n      isArray\n      isNullable\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n    dependencies {\n      id\n      name\n      files {\n        id\n        path\n        symbols {\n          ...InterpSymbolContent\n        }\n      }\n    }\n    errors {\n      ...InterpErrorContent\n    }\n    staleSymbols {\n      id\n      name\n      type\n      symbolType\n      modifier\n      parentId\n      rootTypeTag\n      generated\n    }\n  }\n"
): typeof documents["\n  fragment InterpModuleContent on InterpModule {\n    id\n    name\n    files {\n      id\n      path\n      symbols {\n        ...InterpSymbolContent\n      }\n    }\n    dependencies {\n      id\n      name\n      files {\n        id\n        path\n        symbols {\n          ...InterpSymbolContent\n        }\n      }\n    }\n    errors {\n      ...InterpErrorContent\n    }\n    staleSymbols {\n      id\n      name\n      type\n      symbolType\n      modifier\n      parentId\n      rootTypeTag\n      generated\n    }\n  }\n"];
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
  source: "\n      subscription interpChanged($projectVersionId: GlobalID!) {\n        interpChanged(projectVersionId: $projectVersionId) {\n          ...InterpModuleContent\n        }\n      }\n    "
): typeof documents["\n      subscription interpChanged($projectVersionId: GlobalID!) {\n        interpChanged(projectVersionId: $projectVersionId) {\n          ...InterpModuleContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query systemInfo {\n        systemInfo {\n          version\n          gitCommit\n        }\n      }\n    "
): typeof documents["\n      query systemInfo {\n        systemInfo {\n          version\n          gitCommit\n        }\n      }\n    "];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<
  infer TType,
  any
>
  ? TType
  : never;
