<script lang="ts" setup>
import { graphql, type FragmentType, useFragment } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const FileHeader = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    createdAt
    updatedAt
  }
`);

const props = defineProps<{ file: FragmentType<typeof FileHeader> }>();

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query getFileById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        name
        definitions {
          id
          name
          type
          nameDotType
          createdAt
          updatedAt
          content {
            ... on Task {
              schema
              expectations {
                nameDotType
              }
              templateImplementation {
                nameDotType
              }
              compilations {
                name
              }
            }
            ... on Dataset {
              records {
                data
                index
              }
            }
          }
        }
      }
    }
  `),
  () => ({
    fileId: props.file.id,
  })
);

const definitions = computed(() => {
  return file.value?.file?.definitions || [];
});
</script>

<template>
  <div class="mx-auto flex flex-col gap-1 overflow-y-scroll">
    <div
      v-for="definition in definitions"
      :key="definition.id"
      class="mx-auto w-96 rounded-sm border border-orange-600 bg-white shadow-md shadow-orange-200"
    >
      <span class="m-1 text-sm text-gray-900">
        {{ definition.nameDotType }}
      </span>
      <textarea class="w-full" :value="JSON.stringify(definition.content, null, 2)" />
      <!-- <MonacoEditor :model-value="JSON.stringify(definition.content, null, 2)" /> -->
    </div>
  </div>
</template>
