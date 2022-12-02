<script lang="ts" setup>
import { graphql, useFragment, type FragmentType } from "@/gql";

const TaskContent = graphql(/* GraphQL */ `
  fragment TaskContent on Task {
    id
    schema
    expectations {
      id
      nameDotType
    }
    templateImplementation {
      id
      nameDotType
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof TaskContent> }>();
const content = useFragment(TaskContent, props.content);
</script>
<template>
  <div>
    {{ content.schema }}
    <span v-for="expectation in content.expectations" :key="expectation.id">
      {{ expectation.nameDotType }}
    </span>
  </div>
</template>
