import { graphql, useFragment } from "@/gql";
import { JobStatus, JobType } from "@/gql/graphql";
import { getUpdatedConnectionQuery } from "@/utils/connection";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, type Ref } from "vue";

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
    statusIn?: Ref<JobStatus[] | null>;
    typeIn?: Ref<JobType[] | null>;
  },
  options?: { first?: number; live?: boolean }
) {
  const { result: jobsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query jobs(
        $projectId: GlobalID!
        $projectVersionId: GlobalID!
        $statusIn: [JobStatus!]
        $typeIn: [JobType!]
        $first: Int
      ) {
        jobs(
          projectId: $projectId
          projectVersionId: $projectVersionId
          statusIn: $statusIn
          typeIn: $typeIn
          first: $first
        ) {
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
      statusIn: filter.statusIn ?? ref<JobStatus[]>([JobStatus.Running]),
      typeIn: filter.typeIn ?? ref<JobType[]>([]),
      first: options?.first ?? 25,
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
        typeIn: filter.typeIn ?? ref<JobType[]>([]),
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const job = useFragment(JobContentType, subscriptionData.data.jobsChanged);
        return {
          jobs: getUpdatedConnectionQuery(job, prev.jobs, options.first),
        };
      },
    });
  }

  return {
    totalCount: computed(() => jobsResult.value?.jobs?.totalCount ?? 0),
    jobs: computed(
      () =>
        jobsResult.value?.jobs?.edges
          ?.map((edge: any) => useFragment(JobContentType, edge.node))
          .filter(
            (job) => (filter.statusIn?.value?.length ?? 0) == 0 || filter.statusIn?.value?.includes(job.status)
          ) ?? []
    ),
  };
}
