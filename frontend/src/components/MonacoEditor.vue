<template>
  <div id="container" class="h-40 w-1/2"></div>
</template>
<script lang="ts" setup>
import * as monaco from "monaco-editor";

monaco.editor.create(document.getElementById("container"), {
  value: "def test()",
  language: "python",
});
</script>
