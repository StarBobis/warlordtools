<script setup lang="ts">
import { ref } from "vue";

const isLoading = ref(true);

const onFrameLoad = () => {
    isLoading.value = false;
};
</script>

<template>
  <div class="workshop-page glass-panel">
    <div v-if="isLoading" class="loader">
        <div class="spinner"></div>
        <p>正在加载白菜工坊...</p>
    </div>
    <iframe 
        src="https://poe2ggg.com/" 
        class="webview-frame"
        @load="onFrameLoad"
        frameborder="0"
        allow="clipboard-read; clipboard-write; encrypted-media; picture-in-picture"
        sandbox="allow-forms allow-modals allow-popups allow-presentation allow-same-origin allow-scripts allow-downloads"
    ></iframe>
  </div>
</template>

<style scoped>
.workshop-page {
  height: 100%;
  width: 100%;
  overflow: hidden;
  position: relative;
  border-radius: 0; /* Override default glass-panel radius if needed to fill space */
  display: flex;
  flex-direction: column;
}

.webview-frame {
    flex: 1;
    width: 100%;
    height: 100%;
    border: none;
    background: #111; /* Fallback */
}

.loader {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    color: #888;
    pointer-events: none;
    z-index: 10;
}

.spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255,255,255,0.1);
    border-top-color: #409eff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 12px;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}
</style>
