<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";

const CodeContentFragment = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
    id
    builtinId
    code
    inputSchema {
      ...SchemaElementContentDeep
    }
    outputSchema {
      ...SchemaElementContentDeep
    }
    parameters {
      name
      type
      schema {
        ...SchemaElementContentDeep
      }
    }
    arguments {
      name
      type
      value
      reference {
        id
        nameDotType
      }
    }
  }
`);

const props = defineProps<{
  content: FragmentType<typeof CodeContentFragment>;
  generated: boolean;
  focused: boolean;
}>();
const content = useFragment(CodeContentFragment, props.content);
</script>
<template>
  <div>
    <div v-if="!content.code">builtin {{ content.builtinId }}</div>
    <MonacoEditor v-else :model-value="content.code" language="python" :focused="focused" :readonly="generated" />
  </div>
</template>
