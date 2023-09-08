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
  "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              organizationInvite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n              projectInvite {\n                id\n                project {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.NotificationsDocument,
  "\n    query existingProjectVersionTag($projectId: GlobalID!, $tag: String!) {\n      projectVersionByTag(projectId: $projectId, tag: $tag) {\n        id\n        tag\n      }\n    }\n  ":
    types.ExistingProjectVersionTagDocument,
  "\n    query blankPanelSuggestedFiles($projectVersionId: GlobalID!) {\n      module(id: $projectVersionId) {\n        files(filters: { isVisible: true }) {\n          id\n          ck\n          name\n          deletedAt\n        }\n      }\n    }\n  ":
    types.BlankPanelSuggestedFilesDocument,
  "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        # :fileContentById\n        id\n        ck\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n        issues {\n          ...IssueContent\n        }\n      }\n    }\n  ":
    types.FileContentByIdDocument,
  "\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        parent {\n          id\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  ":
    types.StatementContentByIdDocument,
  "\n    query profileAccessTokens($slug: String!, $includeInactive: Boolean!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          accessTokens(filters: { includeInactive: $includeInactive }) {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                tokenKey\n                createdAt\n                updatedAt\n                expiresAt\n                revokedAt\n                status\n                scopes\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProfileAccessTokensDocument,
  "\n    mutation createAccessToken(\n      $ownerId: GlobalID!\n      $scopes: [AccessTokenScope!]!\n      $expiresAt: DateTime\n      $name: String\n    ) {\n      createAccessToken(input: { ownerId: $ownerId, scopes: $scopes, expiresAt: $expiresAt, name: $name }) {\n        ... on AccessTokenCreatePayload {\n          token\n          accessToken {\n            id\n            name\n            tokenKey\n            createdAt\n            updatedAt\n            expiresAt\n            revokedAt\n            status\n            scopes\n          }\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.CreateAccessTokenDocument,
  "\n    mutation revokeAccessToken($id: GlobalID!) {\n      revokeAccessToken(id: $id) {\n        ... on AccessToken {\n          id\n          revokedAt\n          status\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.RevokeAccessTokenDocument,
  "\n    query organizationMembers($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.OrganizationMembersDocument,
  "\n    query profileSettings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          description\n          canWrite\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          canWrite\n        }\n      }\n    }\n  ":
    types.ProfileSettingsDocument,
  "\n    mutation updateOrganization($id: GlobalID!, $name: String!, $description: String!) {\n      updateOrganization(input: { id: $id, name: $name, description: $description }) {\n        ... on Organization {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.UpdateOrganizationDocument,
  "\n    mutation updateUser($id: GlobalID!, $name: String!, $description: String!) {\n      updateUser(input: { id: $id, name: $name, description: $description }) {\n        ... on User {\n          id\n          name\n          description\n        }\n        ...OperationInfoContent\n      }\n    }\n  ":
    types.UpdateUserDocument,
  "\n    query environment($projectId: GlobalID!) {\n      environment(projectId: $projectId) {\n        ... on Environment {\n          language\n          version\n          platform\n          packages {\n            name\n            version\n          }\n        }\n      }\n      project(id: $projectId) {\n        id\n        usage {\n          recordsActive\n          objectsBytesTotal\n          cacheBytesTotal\n        }\n      }\n    }\n  ":
    types.EnvironmentDocument,
  "\n    query projectVersions($projectId: GlobalID!) {\n      project(id: $projectId) {\n        id\n        head {\n          ...ProjectVersionHeader\n        }\n        versions {\n          totalCount\n          edges {\n            node {\n              ...ProjectVersionHeader\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProjectVersionsDocument,
  "\n      query checkOwnerBySlug($slug: String!) {\n        ownerBySlug(slug: $slug) {\n          ... on Organization {\n            id\n          }\n          ... on User {\n            id\n          }\n        }\n      }\n    ":
    types.CheckOwnerBySlugDocument,
  "\n    query projectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        ...ProjectHeader\n      }\n    }\n  ":
    types.ProjectBySlugDocument,
  "\n    query projectVersionHeader($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ck\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        parents {\n          id\n        }\n        children {\n          id\n        }\n      }\n    }\n  ":
    types.ProjectVersionHeaderDocument,
  "\n    query existingProjectBySlug($owner: String!, $project: String!) {\n      projectBySlug(owner: $owner, project: $project) {\n        id\n        slug\n      }\n    }\n  ":
    types.ExistingProjectBySlugDocument,
  "\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.HomeBenchesDocument,
  "\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            visibility\n            description\n          }\n        }\n      }\n    }\n  ":
    types.FeaturedBenchesDocument,
  "\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  ":
    types.ProfileHomeDocument,
  "\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          memberships {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  ":
    types.SettingsDocument,
  "\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          status\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
    types.MeDocument,
  "\n  fragment ClientContentType on Client {\n    id\n    type\n    deviceName\n    browserName\n    user {\n      id\n      name\n      username\n      email\n    }\n    project {\n      id\n      name\n    }\n    fileId\n    statementId\n    lastSeenAt\n    closedAt\n    active\n    present\n  }\n":
    types.ClientContentTypeFragmentDoc,
  "\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    ":
    types.ConnectedClientsDocument,
  "\n        subscription clientsChanged($projectId: GlobalID, $projectVersionId: GlobalID) {\n          clientsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ...ClientContentType\n          }\n        }\n      ":
    types.ClientsChangedDocument,
  "\n      fragment ClientStatus on Client {\n        id\n        lastSeenAt\n        closedAt\n        active\n        present\n      }\n    ":
    types.ClientStatusFragmentDoc,
  "\n  query searchRecords(\n    $statementId: GlobalID!\n    $query: SearchQuery\n    $sort: [SearchSort!]\n    $after: String\n    $limit: Int\n    $count: Boolean\n  ) {\n    searchRecords(statementId: $statementId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        hasPreviousPage\n        startCursor\n        endCursor\n      }\n      edges {\n        cursor\n        node {\n          id\n          revision\n          createdAt\n          updatedAt\n          deletedAt\n          orderKey\n          value\n        }\n      }\n    }\n  }\n":
    types.SearchRecordsDocument,
  "\n  fragment PageInfo on PageInfo {\n    hasNextPage\n    hasPreviousPage\n    startCursor\n    endCursor\n  }\n":
    types.PageInfoFragmentDoc,
  "\n  fragment OperationInfoContent on OperationInfo {\n    ... on OperationInfo {\n      messages {\n        kind\n        message\n        field\n      }\n    }\n  }\n":
    types.OperationInfoContentFragmentDoc,
  "\n  fragment HasCrudContent on HasCrud {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.HasCrudContentFragmentDoc,
  "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    ck\n    name\n    tag\n    description\n    committed\n    committedAt\n    parents {\n      id\n    }\n    children {\n      id\n    }\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.ProjectVersionHeaderFragmentDoc,
  "\n  fragment ProjectHeader on Project {\n    id\n    createdAt\n    updatedAt\n    name\n    slug\n    head {\n      ...ProjectVersionHeader\n    }\n    visibility\n    accessLevel\n    sharingEnabled\n    sharingToken\n    sharingLevel\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n":
    types.ProjectHeaderFragmentDoc,
  "\n  fragment FileHeader on File {\n    __typename\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    projectVersion {\n      id\n    }\n    deletedAt\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.FileHeaderFragmentDoc,
  "\n  fragment StatementHeader on Statement {\n    __typename\n    id\n    ck\n    type\n    revision\n    name\n    headingLevel\n    text\n    orderKey\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n":
    types.StatementHeaderFragmentDoc,
  "\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    ck\n    revision\n    name\n    key\n    tag\n    hint\n    flags\n    text\n    orderKey\n    referenceCk\n    parent {\n      id\n    }\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.FieldContentFragmentDoc,
  "\n  fragment TaggingContent on Tagging {\n    # :TaggingContent\n    id\n    ck\n    revision\n    key\n    parent {\n      id\n    }\n    referenceCk\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.TaggingContentFragmentDoc,
  "\n  fragment TriggerContent on Trigger {\n    # :TriggerContent\n    id\n    ck\n    revision\n    parent {\n      id\n    }\n    type\n    active\n    mapping\n    timezone\n    scheduleType\n    interval\n    cron\n    runnableCk\n    scopeCk\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.TriggerContentFragmentDoc,
  "\n  fragment StatementContent on Statement {\n    id\n    ck\n    type\n    revision\n    name\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    key\n    text\n    headingLevel\n    code\n    value\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    triggers(filters: { isVisible: true }) {\n      ...TriggerContent\n    }\n    # interp\n    # TODO @Performance: could probably just use module interp state for statement, but would be less responsive on load\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n":
    types.StatementContentFragmentDoc,
  "\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    ck\n    kind\n    type\n    message\n    parent {\n      id\n    }\n  }\n":
    types.IssueContentFragmentDoc,
  "\n  fragment ResolvedFieldContent on ResolvedField {\n    statement {\n      id\n    }\n    fieldCk\n  }\n":
    types.ResolvedFieldContentFragmentDoc,
  "\n  fragment InterpFile on File {\n    # :InterpFile\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    issues {\n      ...IssueContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n":
    types.InterpFileFragmentDoc,
  "\n  fragment InterpStatement on Statement {\n    # :InterpStatement\n    id\n    ck\n    type\n    name\n    text\n    headingLevel\n    revision\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    key\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n":
    types.InterpStatementFragmentDoc,
  "\n      query moduleContentById($projectVersionId: GlobalID!) {\n        module(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files(filters: { isVisible: true }) {\n            ...InterpFile\n            statements(filters: { isVisible: true }) {\n              ...InterpStatement\n            }\n          }\n        }\n      }\n    ":
    types.ModuleContentByIdDocument,
  "\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                organizationInvite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n                projectInvite {\n                  id\n                  project {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    ":
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
  "\n      mutation createFile(\n        $id: GlobalID!\n        $ck: UUID!\n        $projectVersionId: GlobalID!\n        $name: String!\n        $parentId: GlobalID\n      ) {\n        createFile(input: { id: $id, ck: $ck, projectVersionId: $projectVersionId, parentId: $parentId, name: $name }) {\n          ... on File {\n            # :fileContentById\n            # unfortunately can't use FileHeader here for.. error reasons?\n            id\n            ck\n            projectVersion {\n              id\n            }\n            revision\n            name\n            parent {\n              ... on File {\n                id\n              }\n              ... on ProjectVersion {\n                id\n              }\n            }\n            deletedAt\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n            # :InterpFile :InterpStatement\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n              issues {\n                ...IssueContent\n              }\n            }\n            issues {\n              ...IssueContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFileDocument,
  "\n      mutation deleteFile($id: GlobalID!) {\n        deleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFileDocument,
  "\n      mutation softDeleteFile($id: GlobalID!) {\n        softDeleteFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteFileDocument,
  "\n      mutation restoreFile($id: GlobalID!) {\n        restoreFile(input: { id: $id }) {\n          ... on File {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFileDocument,
  "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RenameFileDocument,
  "\n      mutation pasteFile(\n        $sourceId: GlobalID!\n        $targetId: GlobalID!\n        $targetCk: UUID!\n        $targetVersionId: GlobalID!\n        $parentId: GlobalID\n      ) {\n        pasteFile(\n          input: {\n            sourceId: $sourceId\n            targetId: $targetId\n            targetCk: $targetCk\n            targetVersionId: $targetVersionId\n            parentId: $parentId\n          }\n        ) {\n          ... on File {\n            # :fileContentById\n            id\n            ck\n            projectVersion {\n              id\n            }\n            ...FileHeader\n            # :InterpFile :InterpStatement\n            issues {\n              ...IssueContent\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.PasteFileDocument,
  "\n      mutation requestUploadObject(\n        $projectId: GlobalID!\n        $name: String\n        $contentType: String!\n        $contentLength: Int!\n        $sha512: String!\n      ) {\n        requestUploadObject(\n          input: {\n            projectId: $projectId\n            name: $name\n            contentType: $contentType\n            contentLength: $contentLength\n            sha512: $sha512\n          }\n        ) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedPost\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RequestUploadObjectDocument,
  "\n      mutation notifyUploadedObject($id: GlobalID!) {\n        notifyUploadedObject(input: { id: $id }) {\n          ... on RemoteObject {\n            id\n            status\n            name\n            contentType\n            contentLength\n            sha512\n            presignedGet\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.NotifyUploadedObjectDocument,
  "\n      mutation createOrganization($name: String!, $slug: String!) {\n        createOrganization(input: { name: $name, slug: $slug }) {\n          ... on Organization {\n            id\n            name\n            slug\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateOrganizationDocument,
  "\n      mutation createInvites($id: GlobalID!, $emails: [String!]!, $level: OrganizationRole!, $message: String) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateInvitesDocument,
  "\n      mutation cancelInvite($id: GlobalID!) {\n        cancelOrganizationInvite(id: $id) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CancelInviteDocument,
  "\n      mutation createProject($input: ProjectCreateInput!) {\n        createProject(input: $input) {\n          ... on Project {\n            ...ProjectHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateProjectDocument,
  "\n      mutation updateProjectVisibility($id: GlobalID!, $visibility: ProjectVisibility!) {\n        updateProjectVisibility(input: { id: $id, visibility: $visibility }) {\n          ... on Project {\n            id\n            visibility\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectVisibilityDocument,
  "\n      mutation updateProjectSharing(\n        $id: GlobalID!\n        $sharingEnabled: Boolean!\n        $sharingToken: UUID!\n        $sharingLevel: ProjectAccessLevel!\n      ) {\n        updateProjectSharing(\n          input: { id: $id, sharingEnabled: $sharingEnabled, sharingToken: $sharingToken, sharingLevel: $sharingLevel }\n        ) {\n          ... on Project {\n            id\n            sharingEnabled\n            sharingToken\n            sharingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectSharingDocument,
  "\n      mutation updateProjectName($id: GlobalID!, $name: String!) {\n        updateProjectName(input: { id: $id, name: $name }) {\n          ... on Project {\n            id\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateProjectNameDocument,
  "\n      mutation createSecret($projectId: GlobalID!, $name: String, $value: JSON!) {\n        createSecret(input: { projectId: $projectId, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateSecretDocument,
  "\n      mutation updateSecret($id: GlobalID!, $name: String, $value: JSON!) {\n        updateSecret(input: { id: $id, name: $name, value: $value }) {\n          ... on Secret {\n            id\n            createdAt\n            updatedAt\n            sha512\n            name\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSecretDocument,
  "\n      mutation deleteSecret($id: GlobalID!) {\n        deleteSecret(input: { id: $id }) {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteSecretDocument,
  "\n      mutation wakeRuntime($projectVersionId: GlobalID!) {\n        wakeRuntime(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.WakeRuntimeDocument,
  "\n      mutation wakeWorkerSet($projectId: GlobalID!) {\n        wakeWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on WakeWorkerSetPayload {\n            success\n          }\n        }\n      }\n    ":
    types.WakeWorkerSetDocument,
  "\n      mutation restartWorkerSet($projectId: GlobalID!) {\n        restartWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on RestartWorkerSetPayload {\n            success\n          }\n        }\n      }\n    ":
    types.RestartWorkerSetDocument,
  "\n      mutation startRun(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $runId: GlobalID\n        $sessionId: GlobalID\n        $inputs: JSON\n        $keyed: Boolean\n        $block: Float\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            runId: $runId\n            sessionId: $sessionId\n            inputs: $inputs\n            keyed: $keyed\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            error\n            run {\n              ...RunContent\n            }\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      }\n    ":
    types.StartRunDocument,
  "\n      mutation kill($projectVersionId: GlobalID!, $runId: GlobalID!) {\n        killRun(input: { projectVersionId: $projectVersionId, runId: $runId }) {\n          ... on KillRunPayload {\n            success\n            run {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.KillDocument,
  "\n      mutation createStatement(\n        $id: GlobalID!\n        $ck: UUID!\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $key: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            ck: $ck\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            key: $key\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            ck\n            type\n            revision\n            name\n            orderKey\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n            # symbol contents\n            key\n            code\n            text\n            headingLevel\n            value\n            tag\n            flags\n            referenceCk\n            tags(filters: { isVisible: true }) {\n              id\n            }\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            triggers(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              statement {\n                id\n              }\n              fieldCk\n            }\n            issues {\n              id\n            }\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateStatementDocument,
  "\n      mutation updateStatement(\n        $id: GlobalID!\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        updateStatement(\n          input: {\n            id: $id\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            updatedAt\n            name\n            orderKey\n            code\n            text\n            value\n            tag\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementDocument,
  "\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $tag: TypeTag\n        $flags: Int\n        $headingLevel: Int\n      ) {\n        morphStatement(\n          input: { id: $id, type: $type, name: $name, tag: $tag, flags: $flags, headingLevel: $headingLevel }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            tag\n            flags\n            headingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MorphStatementDocument,
  "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveStatementDocument,
  "\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                ... on Statement {\n                  id\n                }\n                ... on File {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
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
  "\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetCks: [UUID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetCks: $targetCks\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchPasteStatementDocument,
  "\n      mutation updateStatementReference($id: GlobalID!, $referenceCk: UUID) {\n        updateStatementReference(input: { id: $id, referenceCk: $referenceCk }) {\n          ... on Statement {\n            id\n            revision\n            referenceCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementReferenceDocument,
  "\n      mutation updateSymbolCode($id: GlobalID!, $code: String) {\n        updateSymbolCode(input: { id: $id, code: $code }) {\n          ... on Statement {\n            id\n            code\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSymbolCodeDocument,
  "\n      mutation updateStatementText($id: GlobalID!, $text: String) {\n        updateStatementText(input: { id: $id, text: $text }) {\n          ... on Statement {\n            id\n            text\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateStatementTextDocument,
  "\n      mutation updateSymbolValue($id: GlobalID!, $value: JSON) {\n        updateSymbolValue(input: { id: $id, value: $value }) {\n          ... on Statement {\n            id\n            value\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateSymbolValueDocument,
  "\n      mutation createRecord($id: GlobalID!, $ck: UUID!, $statementId: GlobalID!, $orderKey: String, $value: JSON!) {\n        createRecord(input: { id: $id, ck: $ck, statementId: $statementId, orderKey: $orderKey, value: $value }) {\n          ... on Record {\n            id\n            ck\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateRecordDocument,
  "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $value: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, value: $value }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateRecordDocument,
  "\n      mutation deleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        deleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteRecordDocument,
  "\n      mutation softDeleteRecord($id: GlobalID!, $statementId: GlobalID!) {\n        softDeleteRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteRecordDocument,
  "\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreRecordDocument,
  "\n      mutation batchSoftDeleteRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchSoftDeleteRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchSoftDeleteRecordDocument,
  "\n      mutation batchRestoreRecord($ids: [GlobalID!]!, $statementId: GlobalID!) {\n        batchRestoreRecord(input: { ids: $ids, statementId: $statementId }) {\n          ... on RecordBatch {\n            records {\n              id\n              deletedAt\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.BatchRestoreRecordDocument,
  "\n      mutation createField(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        createField(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            # should match :FieldContent fragment\n            id\n            ck\n            key\n            orderKey\n            statement {\n              id\n            }\n            parent {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            text\n            referenceCk\n            flags\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateFieldDocument,
  "\n      mutation deleteField($id: GlobalID!) {\n        deleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteFieldDocument,
  "\n      mutation softDeleteField($id: GlobalID!) {\n        softDeleteField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteFieldDocument,
  "\n      mutation restoreField($id: GlobalID!) {\n        restoreField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreFieldDocument,
  "\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            text\n            flags\n            referenceCk\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateFieldDocument,
  "\n      mutation moveField($id: GlobalID!, $orderKey: String!) {\n        moveField(input: { id: $id, orderKey: $orderKey }) {\n          ... on Field {\n            id\n            orderKey\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.MoveFieldDocument,
  "\n      mutation createTagging(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $key: String!\n        $referenceCk: UUID!\n        $metadata: JSON\n      ) {\n        createTagging(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            key: $key\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Tagging {\n            id\n            ck\n            revision\n            key\n            parent {\n              id\n            }\n            referenceCk\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateTaggingDocument,
  "\n      mutation deleteTagging($id: GlobalID!) {\n        deleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.DeleteTaggingDocument,
  "\n      mutation softDeleteTagging($id: GlobalID!) {\n        softDeleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteTaggingDocument,
  "\n      mutation restoreTagging($id: GlobalID!) {\n        restoreTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreTaggingDocument,
  "\n      mutation updateTagging($id: GlobalID!, $metadata: JSON) {\n        updateTagging(input: { id: $id, metadata: $metadata }) {\n          ... on Tagging {\n            id\n            updatedAt\n            revision\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateTaggingDocument,
  "\n      mutation createTrigger(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        createTrigger(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            ck\n            parent {\n              id\n            }\n            revision\n            type\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CreateTriggerDocument,
  "\n      mutation softDeleteTrigger($id: GlobalID!) {\n        softDeleteTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SoftDeleteTriggerDocument,
  "\n      mutation restoreTrigger($id: GlobalID!) {\n        restoreTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.RestoreTriggerDocument,
  "\n      mutation updateTrigger(\n        $id: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        updateTrigger(\n          input: {\n            id: $id\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            updatedAt\n            type\n            revision\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateTriggerDocument,
  "\n      mutation logout {\n        logout {\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.LogoutDocument,
  "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.CompleteSignupDocument,
  "\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.AcceptOrganizationInviteDocument,
  "\n      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {\n        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {\n          ... on ProjectVersion {\n            ...ProjectVersionHeader\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.UpdateVersionDocument,
  "\n      mutation snapshot($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        snapshot(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on SnapshotPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            snapshot {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    ":
    types.SnapshotDocument,
  "\n        query revealSecret($secretId: GlobalID!) {\n          secret(id: $secretId) {\n            ... on Secret {\n              id\n              sha512\n              valueRevealed\n            }\n          }\n        }\n      ":
    types.RevealSecretDocument,
  "\n  fragment WorkerSetContent on WorkerSet {\n    id\n    project {\n      id\n    }\n    region\n    profile\n    sleeping\n    status\n    desiredReplicas\n    targetReplicas\n    availableReplicas\n    readyReplicas\n    lastActiveAt\n  }\n":
    types.WorkerSetContentFragmentDoc,
  "\n  fragment RunHeader on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    runnable {\n      id\n      name\n    }\n    runnableCk\n  }\n":
    types.RunHeaderFragmentDoc,
  "\n  fragment RunContent on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      kind\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    metadata\n    runnable {\n      id\n    }\n    runnableCk\n    # trigger\n    triggerType\n    trigger {\n      id\n      type\n    }\n    triggerUser {\n      id\n      username\n      name\n    }\n    triggerAccessToken {\n      id\n      name\n    }\n  }\n":
    types.RunContentFragmentDoc,
  "\n  fragment LogEntryContent on LogEntry {\n    id\n    createdAt\n    projectVersionId\n    sessionId\n    runnableId\n    runnableCk\n    runId\n    stream\n    level\n    logger\n    message\n    metadata\n  }\n":
    types.LogEntryContentFragmentDoc,
  "\n      query currentRuns($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        currentRuns(projectId: $projectId, projectVersionId: $projectVersionId) {\n          ...OperationInfoContent\n          ... on SessionState {\n            runs {\n              ...RunContent\n            }\n            workerSet {\n              ...WorkerSetContent\n            }\n          }\n        }\n      }\n    ":
    types.CurrentRunsDocument,
  "\n        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID) {\n          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ... on SessionChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on RunsChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on WorkerChange {\n              workerSets {\n                ...WorkerSetContent\n              }\n            }\n          }\n        }\n      ":
    types.SessionsChangedDocument,
  "\n    query searchRuns(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID!\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $rootOnly: Boolean!\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchRuns(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        rootOnly: $rootOnly\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...RunContent\n          }\n          cursor\n        }\n      }\n    }\n  ":
    types.SearchRunsDocument,
  "\n    query runById($id: GlobalID!) {\n      run(id: $id) {\n        ...RunContent\n        descendants {\n          ...RunContent\n        }\n      }\n    }\n  ":
    types.RunByIdDocument,
  "\n    query searchLogs(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchLogs(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...LogEntryContent\n          }\n          cursor\n        }\n      }\n    }\n  ":
    types.SearchLogsDocument,
  "\n        subscription logsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $runnableIds: [GlobalID!]\n          $runnableCks: [UUID!]\n          $sessionId: GlobalID\n          $runId: GlobalID\n        ) {\n          logsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            runnableIds: $runnableIds\n            runnableCks: $runnableCks\n            sessionId: $sessionId\n            runId: $runId\n          ) {\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      ":
    types.LogsChangedDocument,
  "\n      subscription moduleChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        moduleChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n          id\n          clientId\n          mutations {\n            type\n            fileId\n            statementId\n            revision\n            input\n            data {\n              ... on Issue {\n                ...IssueContent\n              }\n              ... on ResolvedField {\n                ...ResolvedFieldContent\n              }\n            }\n          }\n        }\n      }\n    ":
    types.ModuleChangedDocument,
  "\n      subscription projectChanged($projectId: GlobalID!) {\n        projectChanged(projectId: $projectId) {\n          id\n          clientId\n        }\n      }\n    ":
    types.ProjectChangedDocument,
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
  source: "\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              organizationInvite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n              projectInvite {\n                id\n                project {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query notifications($status: NotificationStatus, $notArchived: Boolean, $first: Int) {\n      me {\n        id\n        notifications(filters: { status: $status, notArchived: $notArchived }, first: $first) {\n          totalCount\n          edges {\n            node {\n              id\n              type\n              createdAt\n              readAt\n              archivedAt\n              expiresAt\n              status\n              organizationInvite {\n                id\n                organization {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n              projectInvite {\n                id\n                project {\n                  id\n                  slug\n                  name\n                }\n                level\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query blankPanelSuggestedFiles($projectVersionId: GlobalID!) {\n      module(id: $projectVersionId) {\n        files(filters: { isVisible: true }) {\n          id\n          ck\n          name\n          deletedAt\n        }\n      }\n    }\n  "
): typeof documents["\n    query blankPanelSuggestedFiles($projectVersionId: GlobalID!) {\n      module(id: $projectVersionId) {\n        files(filters: { isVisible: true }) {\n          id\n          ck\n          name\n          deletedAt\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        # :fileContentById\n        id\n        ck\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n        issues {\n          ...IssueContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query fileContentById($fileId: GlobalID!) {\n      file(id: $fileId) {\n        # :fileContentById\n        id\n        ck\n        ...FileHeader\n        statements(filters: { isVisible: true }) {\n          ...StatementContent\n        }\n        issues {\n          ...IssueContent\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        parent {\n          id\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  "
): typeof documents["\n    query statementContentById($statementId: GlobalID!) {\n      statement(id: $statementId) {\n        id\n        projectVersion {\n          id\n        }\n        parent {\n          id\n        }\n        deletedAt\n        ...StatementContent\n      }\n    }\n  "];
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
  source: "\n    query organizationMembers($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query organizationMembers($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on Organization {\n          id\n          canWrite\n          memberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n          invites {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                email\n                emailSentAt\n                user {\n                  id\n                  slug\n                  email\n                  name\n                  username\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
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
  source: "\n    query environment($projectId: GlobalID!) {\n      environment(projectId: $projectId) {\n        ... on Environment {\n          language\n          version\n          platform\n          packages {\n            name\n            version\n          }\n        }\n      }\n      project(id: $projectId) {\n        id\n        usage {\n          recordsActive\n          objectsBytesTotal\n          cacheBytesTotal\n        }\n      }\n    }\n  "
): typeof documents["\n    query environment($projectId: GlobalID!) {\n      environment(projectId: $projectId) {\n        ... on Environment {\n          language\n          version\n          platform\n          packages {\n            name\n            version\n          }\n        }\n      }\n      project(id: $projectId) {\n        id\n        usage {\n          recordsActive\n          objectsBytesTotal\n          cacheBytesTotal\n        }\n      }\n    }\n  "];
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
  source: "\n    query projectVersionHeader($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ck\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        parents {\n          id\n        }\n        children {\n          id\n        }\n      }\n    }\n  "
): typeof documents["\n    query projectVersionHeader($id: GlobalID!) {\n      projectVersion(id: $id) {\n        id\n        ck\n        name\n        tag\n        description\n        createdAt\n        committed\n        committedAt\n        parents {\n          id\n        }\n        children {\n          id\n        }\n      }\n    }\n  "];
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
  source: "\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query homeBenches {\n      me {\n        id\n        slug\n        projects {\n          totalCount\n          edges {\n            node {\n              id\n              name\n              slug\n              path\n              createdAt\n              visibility\n              description\n            }\n          }\n        }\n        organizations {\n          edges {\n            node {\n              projects {\n                totalCount\n                edges {\n                  node {\n                    id\n                    name\n                    slug\n                    path\n                    createdAt\n                    visibility\n                    description\n                  }\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            visibility\n            description\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query featuredBenches {\n      featuredProjects(last: 5) {\n        totalCount\n        edges {\n          node {\n            id\n            name\n            slug\n            path\n            createdAt\n            visibility\n            description\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query profileHome($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          description\n          createdAt\n          canViewDetail\n          canWrite\n          projects {\n            totalCount\n            edges {\n              node {\n                id\n                name\n                slug\n                path\n                createdAt\n                visibility\n                createdAt\n                head {\n                  name\n                  createdAt\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          memberships {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  "
): typeof documents["\n    query settings($slug: String!) {\n      ownerBySlug(slug: $slug) {\n        ... on User {\n          id\n          slug\n          name\n          username\n          bot\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n        ... on Organization {\n          id\n          slug\n          name\n          createdAt\n          updatedAt\n          canViewDetail\n          canWrite\n          memberships {\n            totalCount\n          }\n          accessTokens(filters: { includeInactive: false }) {\n            totalCount\n          }\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          status\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query me {\n        me {\n          id\n          username\n          slug\n          email\n          name\n          createdAt\n          updatedAt\n          status\n          organizationMemberships {\n            totalCount\n            edges {\n              node {\n                id\n                createdAt\n                level\n                organization {\n                  id\n                  name\n                  slug\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
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
  source: "\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query connectedClients(\n        $projectId: GlobalID\n        $projectVersionId: GlobalID\n        $userId: GlobalID\n        $inSameOrganizations: Boolean!\n        $first: Int\n        $active: Boolean\n      ) {\n        clients(\n          projectId: $projectId\n          projectVersionId: $projectVersionId\n          userId: $userId\n          inSameOrganizations: $inSameOrganizations\n          first: $first\n          active: $active\n        ) {\n          totalCount\n          pageInfo {\n            hasNextPage\n            hasPreviousPage\n            startCursor\n            endCursor\n          }\n          edges {\n            node {\n              ...ClientContentType\n            }\n          }\n        }\n      }\n    "];
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
  source: "\n  query searchRecords(\n    $statementId: GlobalID!\n    $query: SearchQuery\n    $sort: [SearchSort!]\n    $after: String\n    $limit: Int\n    $count: Boolean\n  ) {\n    searchRecords(statementId: $statementId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        hasPreviousPage\n        startCursor\n        endCursor\n      }\n      edges {\n        cursor\n        node {\n          id\n          revision\n          createdAt\n          updatedAt\n          deletedAt\n          orderKey\n          value\n        }\n      }\n    }\n  }\n"
): typeof documents["\n  query searchRecords(\n    $statementId: GlobalID!\n    $query: SearchQuery\n    $sort: [SearchSort!]\n    $after: String\n    $limit: Int\n    $count: Boolean\n  ) {\n    searchRecords(statementId: $statementId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        hasPreviousPage\n        startCursor\n        endCursor\n      }\n      edges {\n        cursor\n        node {\n          id\n          revision\n          createdAt\n          updatedAt\n          deletedAt\n          orderKey\n          value\n        }\n      }\n    }\n  }\n"];
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
  source: "\n  fragment HasCrudContent on HasCrud {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment HasCrudContent on HasCrud {\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    ck\n    name\n    tag\n    description\n    committed\n    committedAt\n    parents {\n      id\n    }\n    children {\n      id\n    }\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment ProjectVersionHeader on ProjectVersion {\n    id\n    ck\n    name\n    tag\n    description\n    committed\n    committedAt\n    parents {\n      id\n    }\n    children {\n      id\n    }\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ProjectHeader on Project {\n    id\n    createdAt\n    updatedAt\n    name\n    slug\n    head {\n      ...ProjectVersionHeader\n    }\n    visibility\n    accessLevel\n    sharingEnabled\n    sharingToken\n    sharingLevel\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n"
): typeof documents["\n  fragment ProjectHeader on Project {\n    id\n    createdAt\n    updatedAt\n    name\n    slug\n    head {\n      ...ProjectVersionHeader\n    }\n    visibility\n    accessLevel\n    sharingEnabled\n    sharingToken\n    sharingLevel\n    owner {\n      ... on Organization {\n        id\n        slug\n        name\n      }\n      ... on User {\n        id\n        slug\n        username\n        name\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment FileHeader on File {\n    __typename\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    projectVersion {\n      id\n    }\n    deletedAt\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FileHeader on File {\n    __typename\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    projectVersion {\n      id\n    }\n    deletedAt\n    id\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementHeader on Statement {\n    __typename\n    id\n    ck\n    type\n    revision\n    name\n    headingLevel\n    text\n    orderKey\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"
): typeof documents["\n  fragment StatementHeader on Statement {\n    __typename\n    id\n    ck\n    type\n    revision\n    name\n    headingLevel\n    text\n    orderKey\n    parent {\n      id\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    ck\n    revision\n    name\n    key\n    tag\n    hint\n    flags\n    text\n    orderKey\n    referenceCk\n    parent {\n      id\n    }\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment FieldContent on Field {\n    # :FieldContent\n    id\n    ck\n    revision\n    name\n    key\n    tag\n    hint\n    flags\n    text\n    orderKey\n    referenceCk\n    parent {\n      id\n    }\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment TaggingContent on Tagging {\n    # :TaggingContent\n    id\n    ck\n    revision\n    key\n    parent {\n      id\n    }\n    referenceCk\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment TaggingContent on Tagging {\n    # :TaggingContent\n    id\n    ck\n    revision\n    key\n    parent {\n      id\n    }\n    referenceCk\n    metadata\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment TriggerContent on Trigger {\n    # :TriggerContent\n    id\n    ck\n    revision\n    parent {\n      id\n    }\n    type\n    active\n    mapping\n    timezone\n    scheduleType\n    interval\n    cron\n    runnableCk\n    scopeCk\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment TriggerContent on Trigger {\n    # :TriggerContent\n    id\n    ck\n    revision\n    parent {\n      id\n    }\n    type\n    active\n    mapping\n    timezone\n    scheduleType\n    interval\n    cron\n    runnableCk\n    scopeCk\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment StatementContent on Statement {\n    id\n    ck\n    type\n    revision\n    name\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    key\n    text\n    headingLevel\n    code\n    value\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    triggers(filters: { isVisible: true }) {\n      ...TriggerContent\n    }\n    # interp\n    # TODO @Performance: could probably just use module interp state for statement, but would be less responsive on load\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment StatementContent on Statement {\n    id\n    ck\n    type\n    revision\n    name\n    orderKey\n    parent {\n      id\n    }\n    # symbol contents\n    key\n    text\n    headingLevel\n    code\n    value\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    triggers(filters: { isVisible: true }) {\n      ...TriggerContent\n    }\n    # interp\n    # TODO @Performance: could probably just use module interp state for statement, but would be less responsive on load\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    # crud\n    createdAt\n    updatedAt\n    deletedAt\n    createdBy {\n      id\n    }\n    lastEditedAt\n    lastEditedBy {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    ck\n    kind\n    type\n    message\n    parent {\n      id\n    }\n  }\n"
): typeof documents["\n  fragment IssueContent on Issue {\n    # :IssueContent\n    id\n    ck\n    kind\n    type\n    message\n    parent {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment ResolvedFieldContent on ResolvedField {\n    statement {\n      id\n    }\n    fieldCk\n  }\n"
): typeof documents["\n  fragment ResolvedFieldContent on ResolvedField {\n    statement {\n      id\n    }\n    fieldCk\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpFile on File {\n    # :InterpFile\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    issues {\n      ...IssueContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"
): typeof documents["\n  fragment InterpFile on File {\n    # :InterpFile\n    id\n    ck\n    revision\n    name\n    parent {\n      id\n    }\n    issues {\n      ...IssueContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment InterpStatement on Statement {\n    # :InterpStatement\n    id\n    ck\n    type\n    name\n    text\n    headingLevel\n    revision\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    key\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"
): typeof documents["\n  fragment InterpStatement on Statement {\n    # :InterpStatement\n    id\n    ck\n    type\n    name\n    text\n    headingLevel\n    revision\n    file {\n      id\n    }\n    parent {\n      id\n    }\n    orderKey\n    key\n    tag\n    flags\n    referenceCk\n    tags(filters: { isVisible: true }) {\n      ...TaggingContent\n    }\n    fields(filters: { isVisible: true }) {\n      ...FieldContent\n    }\n    issues {\n      ...IssueContent\n    }\n    resolvedFields {\n      ...ResolvedFieldContent\n    }\n    createdAt\n    updatedAt\n    deletedAt\n    lastEditedAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query moduleContentById($projectVersionId: GlobalID!) {\n        module(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files(filters: { isVisible: true }) {\n            ...InterpFile\n            statements(filters: { isVisible: true }) {\n              ...InterpStatement\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query moduleContentById($projectVersionId: GlobalID!) {\n        module(id: $projectVersionId) {\n          id\n          committed\n          project {\n            path\n            name\n          }\n          files(filters: { isVisible: true }) {\n            ...InterpFile\n            statements(filters: { isVisible: true }) {\n              ...InterpStatement\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                organizationInvite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n                projectInvite {\n                  id\n                  project {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query newNotifications($after: String, $status: NotificationStatus) {\n        me {\n          id\n          notifications(after: $after, filters: { status: $status }) {\n            totalCount\n            edges {\n              node {\n                id\n                type\n                createdAt\n                readAt\n                archivedAt\n                expiresAt\n                status\n                organizationInvite {\n                  id\n                  organization {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n                projectInvite {\n                  id\n                  project {\n                    id\n                    slug\n                    name\n                  }\n                  level\n                }\n              }\n            }\n          }\n        }\n      }\n    "];
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
  source: "\n      mutation createFile(\n        $id: GlobalID!\n        $ck: UUID!\n        $projectVersionId: GlobalID!\n        $name: String!\n        $parentId: GlobalID\n      ) {\n        createFile(input: { id: $id, ck: $ck, projectVersionId: $projectVersionId, parentId: $parentId, name: $name }) {\n          ... on File {\n            # :fileContentById\n            # unfortunately can't use FileHeader here for.. error reasons?\n            id\n            ck\n            projectVersion {\n              id\n            }\n            revision\n            name\n            parent {\n              ... on File {\n                id\n              }\n              ... on ProjectVersion {\n                id\n              }\n            }\n            deletedAt\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n            # :InterpFile :InterpStatement\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n              issues {\n                ...IssueContent\n              }\n            }\n            issues {\n              ...IssueContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createFile(\n        $id: GlobalID!\n        $ck: UUID!\n        $projectVersionId: GlobalID!\n        $name: String!\n        $parentId: GlobalID\n      ) {\n        createFile(input: { id: $id, ck: $ck, projectVersionId: $projectVersionId, parentId: $parentId, name: $name }) {\n          ... on File {\n            # :fileContentById\n            # unfortunately can't use FileHeader here for.. error reasons?\n            id\n            ck\n            projectVersion {\n              id\n            }\n            revision\n            name\n            parent {\n              ... on File {\n                id\n              }\n              ... on ProjectVersion {\n                id\n              }\n            }\n            deletedAt\n            id\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n            # :InterpFile :InterpStatement\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n              issues {\n                ...IssueContent\n              }\n            }\n            issues {\n              ...IssueContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation renameFile($id: GlobalID!, $name: String!) {\n        renameFile(input: { id: $id, name: $name }) {\n          ... on File {\n            id\n            name\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation pasteFile(\n        $sourceId: GlobalID!\n        $targetId: GlobalID!\n        $targetCk: UUID!\n        $targetVersionId: GlobalID!\n        $parentId: GlobalID\n      ) {\n        pasteFile(\n          input: {\n            sourceId: $sourceId\n            targetId: $targetId\n            targetCk: $targetCk\n            targetVersionId: $targetVersionId\n            parentId: $parentId\n          }\n        ) {\n          ... on File {\n            # :fileContentById\n            id\n            ck\n            projectVersion {\n              id\n            }\n            ...FileHeader\n            # :InterpFile :InterpStatement\n            issues {\n              ...IssueContent\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation pasteFile(\n        $sourceId: GlobalID!\n        $targetId: GlobalID!\n        $targetCk: UUID!\n        $targetVersionId: GlobalID!\n        $parentId: GlobalID\n      ) {\n        pasteFile(\n          input: {\n            sourceId: $sourceId\n            targetId: $targetId\n            targetCk: $targetCk\n            targetVersionId: $targetVersionId\n            parentId: $parentId\n          }\n        ) {\n          ... on File {\n            # :fileContentById\n            id\n            ck\n            projectVersion {\n              id\n            }\n            ...FileHeader\n            # :InterpFile :InterpStatement\n            issues {\n              ...IssueContent\n            }\n            statements(filters: { isVisible: true }) {\n              ...StatementContent\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation createInvites($id: GlobalID!, $emails: [String!]!, $level: OrganizationRole!, $message: String) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createInvites($id: GlobalID!, $emails: [String!]!, $level: OrganizationRole!, $message: String) {\n        createOrganizationInvites(input: { id: $id, emails: $emails, level: $level, message: $message }) {\n          ... on Organization {\n            id\n            invites {\n              totalCount\n              edges {\n                node {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation updateProjectSharing(\n        $id: GlobalID!\n        $sharingEnabled: Boolean!\n        $sharingToken: UUID!\n        $sharingLevel: ProjectAccessLevel!\n      ) {\n        updateProjectSharing(\n          input: { id: $id, sharingEnabled: $sharingEnabled, sharingToken: $sharingToken, sharingLevel: $sharingLevel }\n        ) {\n          ... on Project {\n            id\n            sharingEnabled\n            sharingToken\n            sharingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateProjectSharing(\n        $id: GlobalID!\n        $sharingEnabled: Boolean!\n        $sharingToken: UUID!\n        $sharingLevel: ProjectAccessLevel!\n      ) {\n        updateProjectSharing(\n          input: { id: $id, sharingEnabled: $sharingEnabled, sharingToken: $sharingToken, sharingLevel: $sharingLevel }\n        ) {\n          ... on Project {\n            id\n            sharingEnabled\n            sharingToken\n            sharingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation wakeRuntime($projectVersionId: GlobalID!) {\n        wakeRuntime(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation wakeRuntime($projectVersionId: GlobalID!) {\n        wakeRuntime(input: { projectVersionId: $projectVersionId }) {\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation wakeWorkerSet($projectId: GlobalID!) {\n        wakeWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on WakeWorkerSetPayload {\n            success\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation wakeWorkerSet($projectId: GlobalID!) {\n        wakeWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on WakeWorkerSetPayload {\n            success\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restartWorkerSet($projectId: GlobalID!) {\n        restartWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on RestartWorkerSetPayload {\n            success\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation restartWorkerSet($projectId: GlobalID!) {\n        restartWorkerSet(input: { projectId: $projectId }) {\n          ...OperationInfoContent\n          ... on RestartWorkerSetPayload {\n            success\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation startRun(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $runId: GlobalID\n        $sessionId: GlobalID\n        $inputs: JSON\n        $keyed: Boolean\n        $block: Float\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            runId: $runId\n            sessionId: $sessionId\n            inputs: $inputs\n            keyed: $keyed\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            error\n            run {\n              ...RunContent\n            }\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      mutation startRun(\n        $projectVersionId: GlobalID!\n        $runnableId: GlobalID\n        $runId: GlobalID\n        $sessionId: GlobalID\n        $inputs: JSON\n        $keyed: Boolean\n        $block: Float\n        $timeoutSeconds: Int\n      ) {\n        run(\n          input: {\n            projectVersionId: $projectVersionId\n            runnableId: $runnableId\n            runId: $runId\n            sessionId: $sessionId\n            inputs: $inputs\n            keyed: $keyed\n            block: $block\n            timeoutSeconds: $timeoutSeconds\n          }\n        ) {\n          ... on RunState {\n            projectVersionId\n            runnableId\n            success\n            error\n            run {\n              ...RunContent\n            }\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation kill($projectVersionId: GlobalID!, $runId: GlobalID!) {\n        killRun(input: { projectVersionId: $projectVersionId, runId: $runId }) {\n          ... on KillRunPayload {\n            success\n            run {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation kill($projectVersionId: GlobalID!, $runId: GlobalID!) {\n        killRun(input: { projectVersionId: $projectVersionId, runId: $runId }) {\n          ... on KillRunPayload {\n            success\n            run {\n              id\n              status\n              startedAt\n              terminatedAt\n              createdAt\n              updatedAt\n              duration\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createStatement(\n        $id: GlobalID!\n        $ck: UUID!\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $key: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            ck: $ck\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            key: $key\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            ck\n            type\n            revision\n            name\n            orderKey\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n            # symbol contents\n            key\n            code\n            text\n            headingLevel\n            value\n            tag\n            flags\n            referenceCk\n            tags(filters: { isVisible: true }) {\n              id\n            }\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            triggers(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              statement {\n                id\n              }\n              fieldCk\n            }\n            issues {\n              id\n            }\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createStatement(\n        $id: GlobalID!\n        $ck: UUID!\n        $fileId: GlobalID!\n        $parentId: GlobalID\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $key: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        createStatement(\n          input: {\n            id: $id\n            ck: $ck\n            fileId: $fileId\n            parentId: $parentId\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            key: $key\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            ck\n            type\n            revision\n            name\n            orderKey\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n            # symbol contents\n            key\n            code\n            text\n            headingLevel\n            value\n            tag\n            flags\n            referenceCk\n            tags(filters: { isVisible: true }) {\n              id\n            }\n            fields(filters: { isVisible: true }) {\n              id\n            }\n            triggers(filters: { isVisible: true }) {\n              id\n            }\n            # interp\n            resolvedFields {\n              statement {\n                id\n              }\n              fieldCk\n            }\n            issues {\n              id\n            }\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatement(\n        $id: GlobalID!\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        updateStatement(\n          input: {\n            id: $id\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            updatedAt\n            name\n            orderKey\n            code\n            text\n            value\n            tag\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatement(\n        $id: GlobalID!\n        $orderKey: String!\n        $type: StatementType!\n        $name: String\n        $code: String\n        $text: String\n        $value: JSON\n        $tag: TypeTag\n        $flags: Int\n      ) {\n        updateStatement(\n          input: {\n            id: $id\n            orderKey: $orderKey\n            type: $type\n            name: $name\n            code: $code\n            text: $text\n            value: $value\n            tag: $tag\n            flags: $flags\n          }\n        ) {\n          ... on Statement {\n            # should match StatementContent fragment\n            id\n            type\n            revision\n            updatedAt\n            name\n            orderKey\n            code\n            text\n            value\n            tag\n            flags\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $tag: TypeTag\n        $flags: Int\n        $headingLevel: Int\n      ) {\n        morphStatement(\n          input: { id: $id, type: $type, name: $name, tag: $tag, flags: $flags, headingLevel: $headingLevel }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            tag\n            flags\n            headingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation morphStatement(\n        $id: GlobalID!\n        $type: StatementType!\n        $name: String\n        $tag: TypeTag\n        $flags: Int\n        $headingLevel: Int\n      ) {\n        morphStatement(\n          input: { id: $id, type: $type, name: $name, tag: $tag, flags: $flags, headingLevel: $headingLevel }\n        ) {\n          ... on Statement {\n            id\n            revision\n            type\n            name\n            tag\n            flags\n            headingLevel\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $orderKey: String!) {\n        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, orderKey: $orderKey }) {\n          ... on Statement {\n            id\n            orderKey\n            revision\n            file {\n              id\n            }\n            parent {\n              ... on Statement {\n                id\n              }\n              ... on File {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                ... on Statement {\n                  id\n                }\n                ... on File {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchMoveStatement(\n        $ids: [GlobalID!]!\n        $fileId: GlobalID!\n        $parentIds: [GlobalID]!\n        $orderKeys: [String!]!\n      ) {\n        batchMoveStatement(input: { ids: $ids, fileId: $fileId, parentIds: $parentIds, orderKeys: $orderKeys }) {\n          ... on StatementBatch {\n            statements {\n              id\n              orderKey\n              revision\n              file {\n                id\n              }\n              parent {\n                ... on Statement {\n                  id\n                }\n                ... on File {\n                  id\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetCks: [UUID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetCks: $targetCks\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation batchPasteStatement(\n        $sourceIds: [GlobalID!]!\n        $targetIds: [GlobalID!]!\n        $targetCks: [UUID!]!\n        $targetFileId: GlobalID!\n        $targetParentIds: [GlobalID]!\n        $targetOrderKeys: [String!]!\n      ) {\n        batchPasteStatement(\n          input: {\n            sourceIds: $sourceIds\n            targetIds: $targetIds\n            targetCks: $targetCks\n            targetFileId: $targetFileId\n            targetParentIds: $targetParentIds\n            targetOrderKeys: $targetOrderKeys\n          }\n        ) {\n          ... on StatementBatch {\n            statements {\n              id\n              ...StatementContent\n              file {\n                id\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateStatementReference($id: GlobalID!, $referenceCk: UUID) {\n        updateStatementReference(input: { id: $id, referenceCk: $referenceCk }) {\n          ... on Statement {\n            id\n            revision\n            referenceCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateStatementReference($id: GlobalID!, $referenceCk: UUID) {\n        updateStatementReference(input: { id: $id, referenceCk: $referenceCk }) {\n          ... on Statement {\n            id\n            revision\n            referenceCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation updateSymbolValue($id: GlobalID!, $value: JSON) {\n        updateSymbolValue(input: { id: $id, value: $value }) {\n          ... on Statement {\n            id\n            value\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateSymbolValue($id: GlobalID!, $value: JSON) {\n        updateSymbolValue(input: { id: $id, value: $value }) {\n          ... on Statement {\n            id\n            value\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createRecord($id: GlobalID!, $ck: UUID!, $statementId: GlobalID!, $orderKey: String, $value: JSON!) {\n        createRecord(input: { id: $id, ck: $ck, statementId: $statementId, orderKey: $orderKey, value: $value }) {\n          ... on Record {\n            id\n            ck\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createRecord($id: GlobalID!, $ck: UUID!, $statementId: GlobalID!, $orderKey: String, $value: JSON!) {\n        createRecord(input: { id: $id, ck: $ck, statementId: $statementId, orderKey: $orderKey, value: $value }) {\n          ... on Record {\n            id\n            ck\n            createdAt\n            updatedAt\n            deletedAt\n            revision\n            orderKey\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $value: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, value: $value }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateRecord($id: GlobalID!, $statementId: GlobalID!, $value: JSON!) {\n        updateRecord(input: { id: $id, statementId: $statementId, value: $value }) {\n          ... on Record {\n            id\n            updatedAt\n            revision\n            value\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreRecord($id: GlobalID!, $statementId: GlobalID!) {\n        restoreRecord(input: { id: $id, statementId: $statementId }) {\n          ... on Record {\n            id\n            deletedAt\n            revision\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation createField(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        createField(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            # should match :FieldContent fragment\n            id\n            ck\n            key\n            orderKey\n            statement {\n              id\n            }\n            parent {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            text\n            referenceCk\n            flags\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createField(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $key: String!\n        $orderKey: String!\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        createField(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            tag: $tag\n            hint: $hint\n            key: $key\n            orderKey: $orderKey\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            # should match :FieldContent fragment\n            id\n            ck\n            key\n            orderKey\n            statement {\n              id\n            }\n            parent {\n              id\n            }\n            revision\n            name\n            tag\n            hint\n            text\n            referenceCk\n            flags\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation restoreField($id: GlobalID!) {\n        restoreField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreField($id: GlobalID!) {\n        restoreField(input: { id: $id }) {\n          ... on Field {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            text\n            flags\n            referenceCk\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateField(\n        $id: GlobalID!\n        $tag: TypeTag!\n        $hint: TypeHint\n        $name: String\n        $text: String\n        $flags: Int!\n        $referenceCk: UUID\n        $metadata: JSON\n      ) {\n        updateField(\n          input: {\n            id: $id\n            tag: $tag\n            hint: $hint\n            name: $name\n            text: $text\n            flags: $flags\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Field {\n            id\n            tag\n            hint\n            updatedAt\n            revision\n            name\n            text\n            flags\n            referenceCk\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation createTagging(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $key: String!\n        $referenceCk: UUID!\n        $metadata: JSON\n      ) {\n        createTagging(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            key: $key\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Tagging {\n            id\n            ck\n            revision\n            key\n            parent {\n              id\n            }\n            referenceCk\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createTagging(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $key: String!\n        $referenceCk: UUID!\n        $metadata: JSON\n      ) {\n        createTagging(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            key: $key\n            referenceCk: $referenceCk\n            metadata: $metadata\n          }\n        ) {\n          ... on Tagging {\n            id\n            ck\n            revision\n            key\n            parent {\n              id\n            }\n            referenceCk\n            metadata\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation deleteTagging($id: GlobalID!) {\n        deleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation deleteTagging($id: GlobalID!) {\n        deleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteTagging($id: GlobalID!) {\n        softDeleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteTagging($id: GlobalID!) {\n        softDeleteTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreTagging($id: GlobalID!) {\n        restoreTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreTagging($id: GlobalID!) {\n        restoreTagging(input: { id: $id }) {\n          ... on Tagging {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateTagging($id: GlobalID!, $metadata: JSON) {\n        updateTagging(input: { id: $id, metadata: $metadata }) {\n          ... on Tagging {\n            id\n            updatedAt\n            revision\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateTagging($id: GlobalID!, $metadata: JSON) {\n        updateTagging(input: { id: $id, metadata: $metadata }) {\n          ... on Tagging {\n            id\n            updatedAt\n            revision\n            metadata\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation createTrigger(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        createTrigger(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            ck\n            parent {\n              id\n            }\n            revision\n            type\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation createTrigger(\n        $id: GlobalID!\n        $ck: UUID!\n        $statementId: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        createTrigger(\n          input: {\n            id: $id\n            ck: $ck\n            statementId: $statementId\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            ck\n            parent {\n              id\n            }\n            revision\n            type\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n            # crud\n            createdAt\n            updatedAt\n            deletedAt\n            createdBy {\n              id\n            }\n            lastEditedAt\n            lastEditedBy {\n              id\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation softDeleteTrigger($id: GlobalID!) {\n        softDeleteTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation softDeleteTrigger($id: GlobalID!) {\n        softDeleteTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation restoreTrigger($id: GlobalID!) {\n        restoreTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation restoreTrigger($id: GlobalID!) {\n        restoreTrigger(input: { id: $id }) {\n          ... on Trigger {\n            id\n            deletedAt\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation updateTrigger(\n        $id: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        updateTrigger(\n          input: {\n            id: $id\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            updatedAt\n            type\n            revision\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation updateTrigger(\n        $id: GlobalID!\n        $type: TriggerType!\n        $active: Boolean!\n        $mapping: JSON\n        $scheduleType: ScheduleType\n        $timezone: String\n        $interval: Int\n        $cron: String\n        $runnableCk: UUID\n        $scopeCk: UUID\n      ) {\n        updateTrigger(\n          input: {\n            id: $id\n            type: $type\n            active: $active\n            mapping: $mapping\n            scheduleType: $scheduleType\n            timezone: $timezone\n            interval: $interval\n            cron: $cron\n            runnableCk: $runnableCk\n            scopeCk: $scopeCk\n          }\n        ) {\n          ... on Trigger {\n            id\n            updatedAt\n            type\n            revision\n            active\n            mapping\n            scheduleType\n            timezone\n            interval\n            cron\n            runnableCk\n            scopeCk\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation completeSignup($input: UserCompleteSignupInput!) {\n        completeSignup(input: $input) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation acceptOrganizationInvite($id: GlobalID!) {\n        acceptOrganizationInvite(id: $id) {\n          ... on User {\n            id\n            username\n            slug\n            email\n            name\n            createdAt\n            updatedAt\n            status\n            # refetch memberships\n            organizationMemberships {\n              totalCount\n              edges {\n                node {\n                  id\n                  level\n                  organization {\n                    id\n                    name\n                    slug\n                  }\n                }\n              }\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n      mutation snapshot($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        snapshot(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on SnapshotPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            snapshot {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "
): typeof documents["\n      mutation snapshot($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {\n        snapshot(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {\n          ... on SnapshotPayload {\n            project {\n              ...ProjectHeader\n              head {\n                ...ProjectVersionHeader\n              }\n            }\n            snapshot {\n              ...ProjectVersionHeader\n            }\n          }\n          ...OperationInfoContent\n        }\n      }\n    "];
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
  source: "\n  fragment WorkerSetContent on WorkerSet {\n    id\n    project {\n      id\n    }\n    region\n    profile\n    sleeping\n    status\n    desiredReplicas\n    targetReplicas\n    availableReplicas\n    readyReplicas\n    lastActiveAt\n  }\n"
): typeof documents["\n  fragment WorkerSetContent on WorkerSet {\n    id\n    project {\n      id\n    }\n    region\n    profile\n    sleeping\n    status\n    desiredReplicas\n    targetReplicas\n    availableReplicas\n    readyReplicas\n    lastActiveAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment RunHeader on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    runnable {\n      id\n      name\n    }\n    runnableCk\n  }\n"
): typeof documents["\n  fragment RunHeader on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    runnable {\n      id\n      name\n    }\n    runnableCk\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment RunContent on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      kind\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    metadata\n    runnable {\n      id\n    }\n    runnableCk\n    # trigger\n    triggerType\n    trigger {\n      id\n      type\n    }\n    triggerUser {\n      id\n      username\n      name\n    }\n    triggerAccessToken {\n      id\n      name\n    }\n  }\n"
): typeof documents["\n  fragment RunContent on Run {\n    id\n    createdAt\n    updatedAt\n    startedAt\n    terminatedAt\n    duration\n    status\n    projectVersion {\n      id\n      tag\n      name\n    }\n    session {\n      id\n    }\n    root {\n      id\n    }\n    parent {\n      id\n    }\n    inputs\n    outputs\n    errorNice {\n      kind\n      type\n      message\n      traceback {\n        line\n        filename\n        lineno\n        name\n        locals\n      }\n    }\n    metadata\n    runnable {\n      id\n    }\n    runnableCk\n    # trigger\n    triggerType\n    trigger {\n      id\n      type\n    }\n    triggerUser {\n      id\n      username\n      name\n    }\n    triggerAccessToken {\n      id\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n  fragment LogEntryContent on LogEntry {\n    id\n    createdAt\n    projectVersionId\n    sessionId\n    runnableId\n    runnableCk\n    runId\n    stream\n    level\n    logger\n    message\n    metadata\n  }\n"
): typeof documents["\n  fragment LogEntryContent on LogEntry {\n    id\n    createdAt\n    projectVersionId\n    sessionId\n    runnableId\n    runnableCk\n    runId\n    stream\n    level\n    logger\n    message\n    metadata\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      query currentRuns($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        currentRuns(projectId: $projectId, projectVersionId: $projectVersionId) {\n          ...OperationInfoContent\n          ... on SessionState {\n            runs {\n              ...RunContent\n            }\n            workerSet {\n              ...WorkerSetContent\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      query currentRuns($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        currentRuns(projectId: $projectId, projectVersionId: $projectVersionId) {\n          ...OperationInfoContent\n          ... on SessionState {\n            runs {\n              ...RunContent\n            }\n            workerSet {\n              ...WorkerSetContent\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID) {\n          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ... on SessionChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on RunsChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on WorkerChange {\n              workerSets {\n                ...WorkerSetContent\n              }\n            }\n          }\n        }\n      "
): typeof documents["\n        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID) {\n          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n            ... on SessionChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on RunsChange {\n              runs {\n                ...RunContent\n              }\n            }\n            ... on WorkerChange {\n              workerSets {\n                ...WorkerSetContent\n              }\n            }\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query searchRuns(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID!\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $rootOnly: Boolean!\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchRuns(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        rootOnly: $rootOnly\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...RunContent\n          }\n          cursor\n        }\n      }\n    }\n  "
): typeof documents["\n    query searchRuns(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID!\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $rootOnly: Boolean!\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchRuns(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        rootOnly: $rootOnly\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...RunContent\n          }\n          cursor\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query runById($id: GlobalID!) {\n      run(id: $id) {\n        ...RunContent\n        descendants {\n          ...RunContent\n        }\n      }\n    }\n  "
): typeof documents["\n    query runById($id: GlobalID!) {\n      run(id: $id) {\n        ...RunContent\n        descendants {\n          ...RunContent\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n    query searchLogs(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchLogs(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...LogEntryContent\n          }\n          cursor\n        }\n      }\n    }\n  "
): typeof documents["\n    query searchLogs(\n      $projectId: GlobalID!\n      $projectVersionId: GlobalID\n      $runnableIds: [GlobalID!]\n      $runnableCks: [UUID!]\n      $sessionId: GlobalID\n      $runId: GlobalID\n      $query: SearchQuery\n      $sort: [SearchSort!]\n      $after: String\n      $limit: Int\n      $count: Boolean\n    ) {\n      searchLogs(\n        projectId: $projectId\n        projectVersionId: $projectVersionId\n        runnableIds: $runnableIds\n        runnableCks: $runnableCks\n        sessionId: $sessionId\n        runId: $runId\n        query: $query\n        sort: $sort\n        after: $after\n        limit: $limit\n        count: $count\n      ) {\n        totalCount\n        pageInfo {\n          hasNextPage\n          hasPreviousPage\n          startCursor\n          endCursor\n        }\n        edges {\n          node {\n            ...LogEntryContent\n          }\n          cursor\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n        subscription logsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $runnableIds: [GlobalID!]\n          $runnableCks: [UUID!]\n          $sessionId: GlobalID\n          $runId: GlobalID\n        ) {\n          logsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            runnableIds: $runnableIds\n            runnableCks: $runnableCks\n            sessionId: $sessionId\n            runId: $runId\n          ) {\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      "
): typeof documents["\n        subscription logsChanged(\n          $projectId: GlobalID!\n          $projectVersionId: GlobalID!\n          $runnableIds: [GlobalID!]\n          $runnableCks: [UUID!]\n          $sessionId: GlobalID\n          $runId: GlobalID\n        ) {\n          logsChanged(\n            projectId: $projectId\n            projectVersionId: $projectVersionId\n            runnableIds: $runnableIds\n            runnableCks: $runnableCks\n            sessionId: $sessionId\n            runId: $runId\n          ) {\n            logs {\n              ...LogEntryContent\n            }\n          }\n        }\n      "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      subscription moduleChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        moduleChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n          id\n          clientId\n          mutations {\n            type\n            fileId\n            statementId\n            revision\n            input\n            data {\n              ... on Issue {\n                ...IssueContent\n              }\n              ... on ResolvedField {\n                ...ResolvedFieldContent\n              }\n            }\n          }\n        }\n      }\n    "
): typeof documents["\n      subscription moduleChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {\n        moduleChanged(projectId: $projectId, projectVersionId: $projectVersionId) {\n          id\n          clientId\n          mutations {\n            type\n            fileId\n            statementId\n            revision\n            input\n            data {\n              ... on Issue {\n                ...IssueContent\n              }\n              ... on ResolvedField {\n                ...ResolvedFieldContent\n              }\n            }\n          }\n        }\n      }\n    "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(
  source: "\n      subscription projectChanged($projectId: GlobalID!) {\n        projectChanged(projectId: $projectId) {\n          id\n          clientId\n        }\n      }\n    "
): typeof documents["\n      subscription projectChanged($projectId: GlobalID!) {\n        projectChanged(projectId: $projectId) {\n          id\n          clientId\n        }\n      }\n    "];
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
