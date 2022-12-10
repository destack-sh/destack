<script lang="ts" setup>
import SymbolInterface from "@/components/SymbolInterface.vue";
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
        symbols {
          id
          ...SymbolContent
        }
      }
    }
  `),
  () => ({
    fileId: fileHeader.id,
  })
);

const symbols = computed(() => {
  return file.value?.file?.symbols || [];
});
</script>

<template>
  <div class="mx-8 my-3 flex flex-col gap-6">
    <SymbolInterface
      v-for="symbol in symbols"
      :key="symbol.id"
      :file="fileHeader"
      :symbol="symbol"
      class="mx-auto w-full max-w-[1000px]"
    />
  </div>
</template>
