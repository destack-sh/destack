import { graphql, useFragment } from "@/gql";
import type { JobStatus, JobType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, toRef, type Ref } from "vue";

export const JobContentType = graphql(/* GraphQL */ `
  fragment JobContent on Job {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    status
    type
    projectVersion {
      id
    }
  }
`);

export function useJobs(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string>;
    statusIn: Ref<JobStatus[] | null>;
    typeIn: Ref<JobType[] | null>;
  },
  options?: { live?: boolean }
) {
  const { result: jobsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query jobs($projectId: GlobalID!, $projectVersionId: GlobalID!, $statusIn: [JobStatus!], $typeIn: [JobType!]) {
        jobs(projectId: $projectId, projectVersionId: $projectVersionId, statusIn: $statusIn, typeIn: $typeIn) {
          totalCount
          edges {
            node {
              ...JobContent
            }
          }
        }
      }
    `),
    {
      projectId: filter.projectId,
      projectVersionId: filter.projectVersionId,
      typeIn: filter.typeIn,
    }
  );

  if (options?.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription jobsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!, $typeIn: [JobType!]) {
          jobsChanged(projectId: $projectId, projectVersionId: $projectVersionId, typeIn: $typeIn) {
            ...JobContent
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
        typeIn: filter.typeIn,
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const job = useFragment(JobContentType, subscriptionData.data.jobsChanged);
        return {
          jobs: getUpdatedConnectionQuery(job, prev.jobs),
        };
      },
    });
  }

  return {
    totalCount: computed(() => jobsResult.value?.jobs?.totalCount ?? 0),
    jobs: computed(
      () => jobsResult.value?.jobs?.edges?.map((edge: any) => useFragment(JobContentType, edge.node)) ?? []
    ),
  };
}

export function useCurrentJobs(options: { live?: boolean } = { live: true }) {
  const editor = useEditorState();

  return useJobs(
    {
      projectId: toRef(editor, "currentProjectId"),
      projectVersionId: toRef(editor, "currentProjectVersionId"),
      statusIn: ref(null),
      typeIn: ref(null),
    },
    options
  );
}
