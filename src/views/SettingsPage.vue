<script setup lang="ts">
// 空白设置页，供后续重新编写
</script>

<template>
  <div class="settings-page">
    <!-- 请在此添加设置页面内容 -->
  </div>
</template>

<style scoped>
.settings-page {
  position: relative;
  height: 100%;
  width: 100%;
  min-height: 100%;
  display: flex;
  flex-direction: column;
  padding: 18px 20px;
  box-sizing: border-box;
  color: #f5f5f7;
  background: linear-gradient(180deg, rgba(15, 15, 18, 0.7), rgba(12, 12, 14, 0.68));
  backdrop-filter: blur(18px);
  overflow: hidden;
}

/* 覆盖 Element Plus 按下态，避免默认蓝底 */
:deep(.el-button),
:deep(.el-button--primary),
:deep(.el-button:active),
:deep(.el-button--primary:active),
:deep(.el-button--primary.is-active) {
  background-color: rgba(255, 255, 255, 0.08) !important;
  border-color: rgba(255, 255, 255, 0.12) !important;
  box-shadow: none !important;
}

:deep(.el-button:hover),
:deep(.el-button--primary:hover) {
  background-color: rgba(255, 255, 255, 0.12) !important;
}
</style>
