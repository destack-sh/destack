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
  "\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  ":
    types.MatchingUsersDocument,
  "\n    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {\n      projectVersionByTag(projectId: $projectId, tag: $tag) {\n        id\n        tag\n      }\n    }\n  ":
    types.ExistingProjectVersionTagDocument,
  "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.NotificationsDocument,
  "\n    query searchRecords($statementId: GlobalID!, $after: String, $first: Int) {\n      searchRecords(statementId: $statementId, after: $after, first: $first) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          cursor\n          node {\n            id\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            data\n          }\n        }\n      }\n    }\n  ":
    types.SearchRecordsDocument,
  "\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  ":
    types.EmptyEditorSuggestedFilesDocument,
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        id\n        projectVersion {\n          id\n        }\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        file {\n          ...FileHeader\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  ":
    types.StatementContentByIdDocument,
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
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n      query checkOwnerBySlug($slug: String!) {\n        ownerBySlug(slug: $slug) {\n          ... on Organization {\n            id\n          }\n          ... on User {\n            id\n          }\n        }\n      }\n    ":
    types.CheckOwnerBySlugDocument,
  "\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n      }\n    }\n  ":
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
  "\n        query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n          project(id: $projectId) {\n            migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n              isReverse\n              sourceVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              targetVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              refMappings {\n                type\n                sourceId\n                sourceVersionId\n                targetId\n                targetVersionId\n              }\n            }\n          }\n        }\n      ":
    types.ProjectMigrationRefsDocument,
  "\n  fragment ClientContentType on Client {\n    id\n    type\n    deviceName\n    browserName\n    user {\n      id\n      name\n      username\n      email\n    }\n    project {\n      id\n      name\n    }\n    fileId\n    statementId\n    lastSeenAt\n    closedAt\n    active\n    present\n  }\n":
    types.ClientContentTypeFragmentDoc,
  "\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    ":
    types.ConnectedClientsDocument,
  "\n        subscription clientsChanged($projectId: GlobalID, $projectVersionId: GlobalID) {\n          clientsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ...ClientContentType\n          }\n        }\n      ":
    types.ClientsChangedDocument,
  "\n      fragment ClientStatus on Client {\n        id\n        lastSeenAt\n        closedAt\n        active\n        present\n      }\n    ":
    types.ClientStatusFragmentDoc,
  "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    cachedGeneratedAt\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    runnable {\n      id\n      name\n    }\n  }\n":
    types.ExecutionContentFragmentDoc,
  "\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $runnableIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          runnableIds: $runnableIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    ":
    types.ExecutionsDocument,
  "\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $runnableIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            runnableIds: $runnableIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      ":
    types.ExecutionsChangedDocument,
  "\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n":
    types.PageInfoFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    name\n    tag\n    description\n    createdAt\n    committed\n    committedAt\n    parents {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    type\n    visibility\n    name\n    slug\n    createdAt\n    updatedAt\n    canWrite\n    head {\n      ...ProjectVersionHeader\n    }\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    projectVersion {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    commented\n    orderKey\n    parent {\n      id\n    }\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    key\n    tag\n    hint\n    description\n    orderKey\n    reference {\n      id\n    }\n    flags\n  }\n":
    types.FieldContentFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    # interp\n    resolvedFields {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    scope\n    kind\n    type\n    message\n    file {\n      id\n    }\n    statement {\n      id\n    }\n  }\n":
    types.IssueContentFragmentDoc,
  "\n  fragment ResolvedFieldContent on ResolvedField {\n    id\n    statement {\n      id\n    }\n    field {\n      ...FieldContent\n    }\n  }\n":
    types.ResolvedFieldContentFragmentDoc,
  "\n  fragment InterpFile on File {\n    id\n    revision\n    name\n    path\n    directory\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n  }\n":
    types.InterpFileFragmentDoc,
  "\n  fragment InterpStatement on Statement {\n    id\n    type\n    name\n    modifier\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n  }\n":
    types.InterpStatementFragmentDoc,
  "\n  fragment InterpStatementData on Statement {\n    id\n    # TODO @Cleanup: use FieldContent and IssueContent fragments (which can't be found for some reason)\n    resolvedFields {\n      # :FieldContent\n      id\n      createdAt\n      updatedAt\n      deletedAt\n      revision\n      name\n      key\n      tag\n      hint\n      description\n      orderKey\n      reference {\n        id\n      }\n      flags\n    }\n    issues {\n      # :IssueContent\n      id\n      kind\n      scope\n      type\n      message\n      file {\n        id\n      }\n      statement {\n        id\n      }\n    }\n  }\n":
    types.InterpStatementDataFragmentDoc,
  "\n      query module($projectVersionId: GlobalID!) {\n        projectVersion(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files {\n            edges {\n              node {\n                ...InterpFile\n                statements {\n                  ...InterpStatement\n                  issues {\n                    ...IssueContent\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.ModuleDocument,
  "\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                invite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.NewNotificationsDocument,
  "\n      mutation markNotification($id: GlobalID!, $status: NotificationStatus!) {\n        markNotification(input: { id: $id, status: $status }) {\n          ... on Notification {\n            id\n            status\n            readAt\n            archivedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MarkNotificationDocument,
  "\n        query remoteObject($id: GlobalID!) {\n          remoteObject(id: $id) {\n            ... on RemoteObject {\n              id\n              presignedGet\n            }\n          }\n        }\n      ":
    types.RemoteObjectDocument,
  "\n      mutation upsertClient(\n        $id: GlobalID!\n        $type: ClientType!\n        $deviceName: String\n        $browserName: String\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $fileId: GlobalID\n        $statementId: GlobalID\n        $fieldId: GlobalID\n        $recordId: GlobalID\n        $path: String\n      ) {\n        upsertClient(\n          input: {\n            id: $id\n            type: $type\n            deviceName: $deviceName\n            browserName: $browserName\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            fileId: $fileId\n            statementId: $statementId\n            fieldId: $fieldId\n            recordId: $recordId\n            path: $path\n          }\n        ) {\n          ... on Client {\n            id\n            type\n            deviceName\n            browserName\n            projectVersion {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpsertClientDocument,
  "\n      mutation closeClient {\n        closeClient {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CloseClientDocument,
  "\n      mutation updatePresence {\n        updatePresence {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdatePresenceDocument,
  "\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            revision\n            name\n            path\n            parent {\n              id\n            }\n            createdAt\n            updatedAt\n            deletedAt\n            directory\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        deleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation softDeleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!, $path: String!) {\n        renameFile(input: { id: $id, name: $name, path: $path }) {\n          ... on File {\n            id\n            name\n            path\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation requestUploadObject(\n        $projectId: GlobalID!\n        $name: String\n        $contentType: String!\n        $contentLength: Int!\n        $sha512: String!\n      ) {\n        requestUploadObject(\n          input: {\n            projectId: $projectId\n            name: $name\n            contentType: $contentType\n            contentLength: $contentLength\n            sha512: $sha512\n          }\n        ) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedPost\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RequestUploadObjectDocument,
  "\n      mutation notifyUploadedObject($id: GlobalID!) {\n        notifyUploadedObject(input: { id: $id }) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.NotifyUploadedObjectDocument,
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
  "\n      mutation wakeLangserver($projectVersionId: GlobalID!) {\n        langserverWake(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.WakeLangserverDocument,
  "\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $executionId: GlobalID\n        $arguments: JSON\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            executionId: $executionId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n              inputs\n              outputs\n              errorNice {\n                type\n                message\n                traceback {\n                  line\n                  filename\n                  lineno\n                  name\n                  locals\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.RunDocument,
  "\n      mutation cancel($projectVersionId: GlobalID!, $executionId: GlobalID!) {\n        cancelRun(input: { projectVersionId: $projectVersionId, executionId: $executionId }) {\n          ... on CancelRunPayload {\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CancelDocument,
  "\n      mutation createSecret($projectId: GlobalID!, $name: String, $value: JSON!) {\n        createSecret(input: { projectId: $projectId, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateSecretDocument,
  "\n      mutation updateSecret($id: GlobalID!, $name: String, $value: JSON!) {\n        updateSecret(input: { id: $id, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSecretDocument,
  "\n      mutation deleteSecret($id: GlobalID!) {\n        deleteSecret(input: { id: $id }) {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteSecretDocument,
  "\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $modifier: ExpectationModifier\n        $name: String\n        $lang: String\n        $code: String\n        $description: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $commented: Boolean\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            modifier: $modifier\n            name: $name\n            lang: $lang\n            code: $code\n            description: $description\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            commented: $commented\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            name\n            commented\n            modifier\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            # symbol contents\n            lang\n            code\n            description\n            referenceProjectVersion {\n              id\n            }\n            rootTypeTag\n            rootTypeFlags\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              id\n            }\n            issues {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $lang: String\n      ) {\n        morphStatement(\n          input: {\n            id: $id\n            type: $type\n            name: $name\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            lang: $lang\n          }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            rootTypeTag\n            rootTypeFlags\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation updateExpectationModifier($id: GlobalID!, $modifier: ExpectationModifier) {\n        updateSymbolModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateExpectationModifierDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchMoveStatementDocument,
  "\n      mutation renameStatement($id: GlobalID!, $name: String) {\n        renameStatement(input: { id: $id, name: $name }) {\n          ... on Statement {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameStatementDocument,
  "\n      mutation deleteStatement($id: GlobalID!) {\n        deleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteStatementDocument,
  "\n      mutation softDeleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteStatementDocument,
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
  "\n      mutation updateSymbolDescription($id: GlobalID!, $description: String!) {\n        updateSymbolDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSymbolDescriptionDocument,
  "\n      mutation updateSymbolCode($id: GlobalID!, $code: String) {\n        updateSymbolCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSymbolCodeDocument,
  "\n      mutation updateStatementText($id: GlobalID!, $text: String) {\n        updateStatementText(input: { id: $id, text: $text }) {\n          ... on Statement {\n            id\n            text\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementTextDocument,
  "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Record {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statementId\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateRecordDocument,
  "\n              fragment _orderKey on Record {\n                orderKey\n              }\n            ":
    types._OrderKeyFragmentDoc,
  "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateRecordDocument,
  "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteRecordDocument,
  "\n      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        softDeleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteRecordDocument,
  "\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreRecordDocument,
  "\n      mutation batchSoftDeleteRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchSoftDeleteRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchSoftDeleteRecordDocument,
  "\n      mutation batchRestoreRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchRestoreRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchRestoreRecordDocument,
  "\n      mutation createField(\n        $id: GlobalID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String!\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        createField(\n          input: {\n            id: $id\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            # should match FieldContent fragment\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            key\n            orderKey\n            statement {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            description\n            reference {\n              id\n            }\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFieldDocument,
  "\n      mutation deleteField($id: GlobalID!) {\n        deleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFieldDocument,
  "\n      mutation softDeleteField($id: GlobalID!) {\n        softDeleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteFieldDocument,
  "\n      mutation restoreField($id: GlobalID!) {\n        restoreStatementField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFieldDocument,
  "\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            description\n            flags\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateFieldDocument,
  "\n      mutation moveField($id: GlobalID!, $orderKey: String!) {\n        moveField(input: { id: $id, orderKey: $orderKey }) {\n          ... on Field {\n            id\n            orderKey\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveFieldDocument,
  "\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.LogoutDocument,
  "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CompleteSignupDocument,
  "\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            completedSignup\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.AcceptOrganizationInviteDocument,
  "\n      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {\n        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {\n          ... on ProjectVersion {\n            ...ProjectVersionHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateVersionDocument,
  "\n      mutation commit($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CommitDocument,
  "\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreDocument,
  "\n        query revealSecret($secretId: GlobalID!) {\n          secret(id: $secretId) {\n            ... on Secret {\n              id\n              sha512\n              valueRevealed\n            }\n          }\n        }\n      ":
    types.RevealSecretDocument,
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
  source: "\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query matchingUsers($slug: String, $email: String) {\n      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {\n        totalCount\n        edges {\n          node {\n            id\n            slug\n            username\n            email\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              invite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query searchRecords($statementId: GlobalID!, $after: String, $first: Int) {\n      searchRecords(statementId: $statementId, after: $after, first: $first) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          cursor\n          node {\n            id\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            data\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query searchRecords($statementId: GlobalID!, $after: String, $first: Int) {\n      searchRecords(statementId: $statementId, after: $after, first: $first) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          cursor\n          node {\n            id\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            orderKey\n            data\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {\n      projectVersion(id: $projectVersionId) {\n        files(filters: { isVisible: true }, last: $last) {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              path\n              deletedAt\n              directory\n            }\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        file {\n          ...FileHeader\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  "
): typeof documents["\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        file {\n          ...FileHeader\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  "];
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
  source: "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n      }\n    }\n  "
): typeof documents["\n    query projectVersionContent($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        id\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n      }\n    }\n  "];
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
  source: "\n        query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n          project(id: $projectId) {\n            migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n              isReverse\n              sourceVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              targetVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              refMappings {\n                type\n                sourceId\n                sourceVersionId\n                targetId\n                targetVersionId\n              }\n            }\n          }\n        }\n      "
): typeof documents["\n        query projectMigrationRefs($projectId: GlobalID!, $sourceVersionId: GlobalID!, $targetVersionId: GlobalID!) {\n          project(id: $projectId) {\n            migrationMappings(sourceVersionId: $sourceVersionId, targetVersionId: $targetVersionId) {\n              isReverse\n              sourceVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              targetVersion {\n                id\n                createdAt\n                tag\n                name\n              }\n              refMappings {\n                type\n                sourceId\n                sourceVersionId\n                targetId\n                targetVersionId\n              }\n            }\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ClientContentType on Client {\n    id\n    type\n    deviceName\n    browserName\n    user {\n      id\n      name\n      username\n      email\n    }\n    project {\n      id\n      name\n    }\n    fileId\n    statementId\n    lastSeenAt\n    closedAt\n    active\n    present\n  }\n"
): typeof documents["\n  fragment ClientContentType on Client {\n    id\n    type\n    deviceName\n    browserName\n    user {\n      id\n      name\n      username\n      email\n    }\n    project {\n      id\n      name\n    }\n    fileId\n    statementId\n    lastSeenAt\n    closedAt\n    active\n    present\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription clientsChanged($projectId: GlobalID, $projectVersionId: GlobalID) {\n          clientsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ...ClientContentType\n          }\n        }\n      "
): typeof documents["\n        subscription clientsChanged($projectId: GlobalID, $projectVersionId: GlobalID) {\n          clientsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ...ClientContentType\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      fragment ClientStatus on Client {\n        id\n        lastSeenAt\n        closedAt\n        active\n        present\n      }\n    "
): typeof documents["\n      fragment ClientStatus on Client {\n        id\n        lastSeenAt\n        closedAt\n        active\n        present\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    cachedGeneratedAt\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    runnable {\n      id\n      name\n    }\n  }\n"
): typeof documents["\n  fragment ExecutionContent on Execution {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    cachedDuration\n    cachedGeneratedAt\n    status\n    triggerType\n    projectVersion {\n      id\n      tag\n      name\n    }\n    user {\n      id\n      slug\n    }\n    accessToken {\n      id\n      name\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    runnable {\n      id\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $runnableIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          runnableIds: $runnableIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "
): typeof documents["\n      query executions(\n        $projectId: GlobalID!\n        $projectVersionId: GlobalID\n        $includeAncestorVersions: Boolean\n        $runnableIds: [GlobalID!]\n        $rootIdNull: Boolean\n        $first: Int\n        $last: Int\n      ) {\n        executions(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          includeAncestorVersions: $includeAncestorVersions\n          runnableIds: $runnableIds\n          rootIdNull: $rootIdNull\n          first: $first\n          last: $last\n        ) {\n          totalCount\n          edges {\n            cursor\n            node {\n              ...ExecutionContent\n              descendants {\n                ...ExecutionContent\n              }\n            }\n          }\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $runnableIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            runnableIds: $runnableIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "
): typeof documents["\n        subscription executionsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID\n          $includeAncestorVersions: Boolean\n          $runnableIds: [GlobalID!]\n          $rootIdNull: Boolean\n        ) {\n          executionsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            includeAncestorVersions: $includeAncestorVersions\n            runnableIds: $runnableIds\n            rootIdNull: $rootIdNull\n          ) {\n            ...ExecutionContent\n          }\n        }\n      "];
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
  source: "\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    projectVersion {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    id\n    revision\n    name\n    path\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    directory\n    projectVersion {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    commented\n    orderKey\n    parent {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    modifier\n    name\n    commented\n    orderKey\n    parent {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    key\n    tag\n    hint\n    description\n    orderKey\n    reference {\n      id\n    }\n    flags\n  }\n"
): typeof documents["\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    revision\n    name\n    key\n    tag\n    hint\n    description\n    orderKey\n    reference {\n      id\n    }\n    flags\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    # interp\n    resolvedFields {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    type\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    name\n    commented\n    modifier\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    lang\n    code\n    description\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    # interp\n    resolvedFields {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    scope\n    kind\n    type\n    message\n    file {\n      id\n    }\n    statement {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    scope\n    kind\n    type\n    message\n    file {\n      id\n    }\n    statement {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ResolvedFieldContent on ResolvedField {\n    id\n    statement {\n      id\n    }\n    field {\n      ...FieldContent\n    }\n  }\n"
): typeof documents["\n  fragment ResolvedFieldContent on ResolvedField {\n    id\n    statement {\n      id\n    }\n    field {\n      ...FieldContent\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpFile on File {\n    id\n    revision\n    name\n    path\n    directory\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n  }\n"
): typeof documents["\n  fragment InterpFile on File {\n    id\n    revision\n    name\n    path\n    directory\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpStatement on Statement {\n    id\n    type\n    name\n    modifier\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n  }\n"
): typeof documents["\n  fragment InterpStatement on Statement {\n    id\n    type\n    name\n    modifier\n    revision\n    createdAt\n    updatedAt\n    deletedAt\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    referenceProjectVersion {\n      id\n    }\n    rootTypeTag\n    rootTypeFlags\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpStatementData on Statement {\n    id\n    # TODO @Cleanup: use FieldContent and IssueContent fragments (which can't be found for some reason)\n    resolvedFields {\n      # :FieldContent\n      id\n      createdAt\n      updatedAt\n      deletedAt\n      revision\n      name\n      key\n      tag\n      hint\n      description\n      orderKey\n      reference {\n        id\n      }\n      flags\n    }\n    issues {\n      # :IssueContent\n      id\n      kind\n      scope\n      type\n      message\n      file {\n        id\n      }\n      statement {\n        id\n      }\n    }\n  }\n"
): typeof documents["\n  fragment InterpStatementData on Statement {\n    id\n    # TODO @Cleanup: use FieldContent and IssueContent fragments (which can't be found for some reason)\n    resolvedFields {\n      # :FieldContent\n      id\n      createdAt\n      updatedAt\n      deletedAt\n      revision\n      name\n      key\n      tag\n      hint\n      description\n      orderKey\n      reference {\n        id\n      }\n      flags\n    }\n    issues {\n      # :IssueContent\n      id\n      kind\n      scope\n      type\n      message\n      file {\n        id\n      }\n      statement {\n        id\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query module($projectVersionId: GlobalID!) {\n        projectVersion(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files {\n            edges {\n              node {\n                ...InterpFile\n                statements {\n                  ...InterpStatement\n                  issues {\n                    ...IssueContent\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query module($projectVersionId: GlobalID!) {\n        projectVersion(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files {\n            edges {\n              node {\n                ...InterpFile\n                statements {\n                  ...InterpStatement\n                  issues {\n                    ...IssueContent\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
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
  source: "\n        query remoteObject($id: GlobalID!) {\n          remoteObject(id: $id) {\n            ... on RemoteObject {\n              id\n              presignedGet\n            }\n          }\n        }\n      "
): typeof documents["\n        query remoteObject($id: GlobalID!) {\n          remoteObject(id: $id) {\n            ... on RemoteObject {\n              id\n              presignedGet\n            }\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation upsertClient(\n        $id: GlobalID!\n        $type: ClientType!\n        $deviceName: String\n        $browserName: String\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $fileId: GlobalID\n        $statementId: GlobalID\n        $fieldId: GlobalID\n        $recordId: GlobalID\n        $path: String\n      ) {\n        upsertClient(\n          input: {\n            id: $id\n            type: $type\n            deviceName: $deviceName\n            browserName: $browserName\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            fileId: $fileId\n            statementId: $statementId\n            fieldId: $fieldId\n            recordId: $recordId\n            path: $path\n          }\n        ) {\n          ... on Client {\n            id\n            type\n            deviceName\n            browserName\n            projectVersion {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation upsertClient(\n        $id: GlobalID!\n        $type: ClientType!\n        $deviceName: String\n        $browserName: String\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $fileId: GlobalID\n        $statementId: GlobalID\n        $fieldId: GlobalID\n        $recordId: GlobalID\n        $path: String\n      ) {\n        upsertClient(\n          input: {\n            id: $id\n            type: $type\n            deviceName: $deviceName\n            browserName: $browserName\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            fileId: $fileId\n            statementId: $statementId\n            fieldId: $fieldId\n            recordId: $recordId\n            path: $path\n          }\n        ) {\n          ... on Client {\n            id\n            type\n            deviceName\n            browserName\n            projectVersion {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation closeClient {\n        closeClient {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation closeClient {\n        closeClient {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updatePresence {\n        updatePresence {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updatePresence {\n        updatePresence {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            revision\n            name\n            path\n            parent {\n              id\n            }\n            createdAt\n            updatedAt\n            deletedAt\n            directory\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      # path is only used for optimistic responses\n      mutation createFile(\n        $id: GlobalID\n        $projectVersionId: GlobalID!\n        $name: String!\n        $directory: Boolean\n        $parentId: GlobalID\n        $path: String!\n      ) {\n        createFile(\n          input: {\n            id: $id\n            projectVersionId: $projectVersionId\n            parentId: $parentId\n            name: $name\n            directory: $directory\n            path: $path\n          }\n        ) {\n          ... on File {\n            id\n            revision\n            name\n            path\n            parent {\n              id\n            }\n            createdAt\n            updatedAt\n            deletedAt\n            directory\n            projectVersion {\n              id\n            }\n            statements(filters: { isVisible: true }) {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteFile($id: GlobalID!) {\n        deleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteFile($id: GlobalID!) {\n        deleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation requestUploadObject(\n        $projectId: GlobalID!\n        $name: String\n        $contentType: String!\n        $contentLength: Int!\n        $sha512: String!\n      ) {\n        requestUploadObject(\n          input: {\n            projectId: $projectId\n            name: $name\n            contentType: $contentType\n            contentLength: $contentLength\n            sha512: $sha512\n          }\n        ) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedPost\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation requestUploadObject(\n        $projectId: GlobalID!\n        $name: String\n        $contentType: String!\n        $contentLength: Int!\n        $sha512: String!\n      ) {\n        requestUploadObject(\n          input: {\n            projectId: $projectId\n            name: $name\n            contentType: $contentType\n            contentLength: $contentLength\n            sha512: $sha512\n          }\n        ) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedPost\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation notifyUploadedObject($id: GlobalID!) {\n        notifyUploadedObject(input: { id: $id }) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation notifyUploadedObject($id: GlobalID!) {\n        notifyUploadedObject(input: { id: $id }) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation wakeLangserver($projectVersionId: GlobalID!) {\n        langserverWake(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation wakeLangserver($projectVersionId: GlobalID!) {\n        langserverWake(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $executionId: GlobalID\n        $arguments: JSON\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            executionId: $executionId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n              inputs\n              outputs\n              errorNice {\n                type\n                message\n                traceback {\n                  line\n                  filename\n                  lineno\n                  name\n                  locals\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation run(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $buildId: GlobalID\n        $executionId: GlobalID\n        $arguments: JSON\n        $block: Boolean\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            buildId: $buildId\n            executionId: $executionId\n            arguments: $arguments\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n              inputs\n              outputs\n              errorNice {\n                type\n                message\n                traceback {\n                  line\n                  filename\n                  lineno\n                  name\n                  locals\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation cancel($projectVersionId: GlobalID!, $executionId: GlobalID!) {\n        cancelRun(input: { projectVersionId: $projectVersionId, executionId: $executionId }) {\n          ... on CancelRunPayload {\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation cancel($projectVersionId: GlobalID!, $executionId: GlobalID!) {\n        cancelRun(input: { projectVersionId: $projectVersionId, executionId: $executionId }) {\n          ... on CancelRunPayload {\n            success\n            execution {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n              cachedGeneratedAt\n              cachedDuration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createSecret($projectId: GlobalID!, $name: String, $value: JSON!) {\n        createSecret(input: { projectId: $projectId, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createSecret($projectId: GlobalID!, $name: String, $value: JSON!) {\n        createSecret(input: { projectId: $projectId, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateSecret($id: GlobalID!, $name: String, $value: JSON!) {\n        updateSecret(input: { id: $id, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateSecret($id: GlobalID!, $name: String, $value: JSON!) {\n        updateSecret(input: { id: $id, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteSecret($id: GlobalID!) {\n        deleteSecret(input: { id: $id }) {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteSecret($id: GlobalID!) {\n        deleteSecret(input: { id: $id }) {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $modifier: ExpectationModifier\n        $name: String\n        $lang: String\n        $code: String\n        $description: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $commented: Boolean\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            modifier: $modifier\n            name: $name\n            lang: $lang\n            code: $code\n            description: $description\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            commented: $commented\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            name\n            commented\n            modifier\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            # symbol contents\n            lang\n            code\n            description\n            referenceProjectVersion {\n              id\n            }\n            rootTypeTag\n            rootTypeFlags\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              id\n            }\n            issues {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement(\n        $id: GlobalID\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $modifier: ExpectationModifier\n        $name: String\n        $lang: String\n        $code: String\n        $description: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $commented: Boolean\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            modifier: $modifier\n            name: $name\n            lang: $lang\n            code: $code\n            description: $description\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            commented: $commented\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            createdAt\n            updatedAt\n            deletedAt\n            name\n            commented\n            modifier\n            orderKey\n            file {\n              id\n            }\n            parent {\n              id\n            }\n            # symbol contents\n            lang\n            code\n            description\n            referenceProjectVersion {\n              id\n            }\n            rootTypeTag\n            rootTypeFlags\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              id\n            }\n            issues {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $lang: String\n      ) {\n        morphStatement(\n          input: {\n            id: $id\n            type: $type\n            name: $name\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            lang: $lang\n          }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            rootTypeTag\n            rootTypeFlags\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $rootTypeTag: TypeTag\n        $rootTypeFlags: Int\n        $lang: String\n      ) {\n        morphStatement(\n          input: {\n            id: $id\n            type: $type\n            name: $name\n            rootTypeTag: $rootTypeTag\n            rootTypeFlags: $rootTypeFlags\n            lang: $lang\n          }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            rootTypeTag\n            rootTypeFlags\n            lang\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateExpectationModifier($id: GlobalID!, $modifier: ExpectationModifier) {\n        updateSymbolModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateExpectationModifier($id: GlobalID!, $modifier: ExpectationModifier) {\n        updateSymbolModifier(input: { id: $id, modifier: $modifier }) {\n          ... on Statement {\n            id\n            modifier\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation deleteStatement($id: GlobalID!) {\n        deleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteStatement($id: GlobalID!) {\n        deleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteStatement($id: GlobalID!) {\n        softDeleteStatement(input: { id: $id }) {\n          ... on Statement {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation updateSymbolDescription($id: GlobalID!, $description: String!) {\n        updateSymbolDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateSymbolDescription($id: GlobalID!, $description: String!) {\n        updateSymbolDescription(input: { id: $id, description: $description }) {\n          ... on Statement {\n            id\n            description\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateSymbolCode($id: GlobalID!, $code: String) {\n        updateSymbolCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateSymbolCode($id: GlobalID!, $code: String) {\n        updateSymbolCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementText($id: GlobalID!, $text: String) {\n        updateStatementText(input: { id: $id, text: $text }) {\n          ... on Statement {\n            id\n            text\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementText($id: GlobalID!, $text: String) {\n        updateStatementText(input: { id: $id, text: $text }) {\n          ... on Statement {\n            id\n            text\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Record {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statementId\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createRecord($id: GlobalID!, $statementId: GlobalID!, $orderKey: String!, $data: JSON!) {\n        createRecord(input: { id: $id, statementId: $statementId, orderKey: $orderKey, data: $data }) {\n          ... on Record {\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            data\n            statementId\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n              fragment _orderKey on Record {\n                orderKey\n              }\n            "
): typeof documents["\n              fragment _orderKey on Record {\n                orderKey\n              }\n            "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $data: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, data: $data }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            data\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        softDeleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        softDeleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation batchSoftDeleteRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchSoftDeleteRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchSoftDeleteRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchSoftDeleteRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation batchRestoreRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchRestoreRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchRestoreRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchRestoreRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createField(\n        $id: GlobalID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String!\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        createField(\n          input: {\n            id: $id\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            # should match FieldContent fragment\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            key\n            orderKey\n            statement {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            description\n            reference {\n              id\n            }\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createField(\n        $id: GlobalID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String!\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        createField(\n          input: {\n            id: $id\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            # should match FieldContent fragment\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            key\n            orderKey\n            statement {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            description\n            reference {\n              id\n            }\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteField($id: GlobalID!) {\n        deleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteField($id: GlobalID!) {\n        deleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteField($id: GlobalID!) {\n        softDeleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteField($id: GlobalID!) {\n        softDeleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreField($id: GlobalID!) {\n        restoreStatementField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreField($id: GlobalID!) {\n        restoreStatementField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            description\n            flags\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $description: String\n        $flags: Int!\n        $referenceId: GlobalID\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            description: $description\n            flags: $flags\n            referenceId: $referenceId\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            description\n            flags\n            reference {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation moveField($id: GlobalID!, $orderKey: String!) {\n        moveField(input: { id: $id, orderKey: $orderKey }) {\n          ... on Field {\n            id\n            orderKey\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation moveField($id: GlobalID!, $orderKey: String!) {\n        moveField(input: { id: $id, orderKey: $orderKey }) {\n          ... on Field {\n            id\n            orderKey\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation commit($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation commit($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        commit(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restore($projectVersionId: GlobalID!) {\n        restore(input: { projectVersionId: $projectVersionId }) {\n          ... on CommitPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            committedVersion {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        query revealSecret($secretId: GlobalID!) {\n          secret(id: $secretId) {\n            ... on Secret {\n              id\n              sha512\n              valueRevealed\n            }\n          }\n        }\n      "
): typeof documents["\n        query revealSecret($secretId: GlobalID!) {\n          secret(id: $secretId) {\n            ... on Secret {\n              id\n              sha512\n              valueRevealed\n            }\n          }\n        }\n      "];
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
