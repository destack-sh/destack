<script lang="ts" setup>
import SymbolDefinitionInterface from "@/components/SymbolDefinitionInterface.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { FileHeaderType } from "@/utils/fragments";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ file: FragmentType<typeof FileHeaderType> }>();
const fileHeader = useFragment(FileHeaderType, props.file);

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        ...FileHeader
        definitions {
          id
          ...SymbolDefinitionContent
        }
      }
    }
  `),
  () => ({
    fileId: fileHeader.id,
  })
);

const definitions = computed(() => {
  return file.value?.file?.definitions || [];
});
</script>

<template>
  <div class="mx-8 my-3 flex flex-col gap-6">
    <SymbolDefinitionInterface
      v-for="definition in definitions"
      :key="definition.id"
      :file="fileHeader"
      :definition="definition"
      class="mx-auto w-full max-w-[1000px]"
    />
  </div>
</template>
