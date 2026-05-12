<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { parseItemText, type ParsedItem } from '../utils/ItemParser';

// State
const manualText = ref('');
const parsedItem = ref<ParsedItem | null>(null);
const statusMsg = ref('');
const statusType = ref<'info' | 'success' | 'error'>('info');
const isSearching = ref(false);
const searchResult = ref<{ url: string; total?: number } | null>(null);
const searchHistory = ref<{ name: string; type: string; time: number; server: string }[]>([]);

// Settings
const server = ref<'cn' | 'intl'>('cn');
const league = ref('Standard');
const sessionCookieCn = ref('');
const sessionCookieIntl = ref('');

const serverLabel: Record<string, string> = {
  cn: '国服',
  intl: '国际服',
};

const leagueOptions = ['Standard', 'Hardcore', 'Settlers', 'Necropolis', 'Affliction'];

function showStatus(msg: string, type: 'info' | 'success' | 'error' = 'info') {
  statusMsg.value = msg;
  statusType.value = type;
  // Longer duration for errors so user can read instructions
  const duration = type === 'error' ? 8000 : 4000;
  setTimeout(() => {
    if (statusMsg.value === msg) statusMsg.value = '';
  }, duration);
}

function handleParse(text?: string) {
  const input = text ?? manualText.value;
  if (!input.trim()) {
    parsedItem.value = null;
    return;
  }
  const item = parseItemText(input);
  if (item) {
    parsedItem.value = item;
    showStatus('物品解析成功', 'success');
  } else {
    showStatus('无法识别物品格式，请确认复制的是 PoE 物品', 'error');
  }
}

watch(manualText, () => {
  searchResult.value = null;
  handleParse();
});

// Global shortcut listener
let unlisten: (() => void) | null = null;

onMounted(async () => {
  loadHistory();
  try {
    unlisten = await listen<{ text: string; sticky: boolean }>('price-check-triggered', (event) => {
      const { text } = event.payload;
      if (text && text.trim()) {
        manualText.value = text;
        handleParse(text);
        showStatus('快捷键查价触发 — ' + (event.payload.sticky ? '固定模式' : '快速模式'), 'info');
      }
    });
    showStatus('已就绪 — 游戏中按 Ctrl+D 查价', 'info');
  } catch (e) {
    console.error('Failed to register event listener:', e);
    showStatus('快捷键注册失败，请重启应用', 'error');
  }
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

// Trade search
async function searchTrade() {
  if (!parsedItem.value) return;

  isSearching.value = true;
  searchResult.value = null;

  const item = parsedItem.value;
  const isNamed = item.rarity === 'Unique' || item.rarity === '传奇';
  const searchName = isNamed ? item.name : '';
  const searchType = item.baseType || item.name;

  try {
    const cookie = server.value === 'cn' ? sessionCookieCn.value : sessionCookieIntl.value;

    const result = await invoke<{ url: string; total: number | null }>('search_trade', {
      itemName: searchName,
      itemType: searchType,
      league: league.value,
      server: server.value,
      sessionCookie: cookie || '',
    });

    searchResult.value = {
      url: result.url,
      total: result.total ?? undefined,
    };

    searchHistory.value.unshift({
      name: searchName || searchType,
      type: searchType,
      time: Date.now(),
      server: server.value,
    });
    if (searchHistory.value.length > 20) {
      searchHistory.value = searchHistory.value.slice(0, 20);
    }
    saveHistory();

    showStatus(`搜索完成 — ${result.total ?? '?'} 个结果`, 'success');
  } catch (e) {
    showStatus(`搜索失败: ${e}`, 'error');
  } finally {
    isSearching.value = false;
  }
}

function openTradeUrl(url: string) {
  invoke('open_file_cmd', { path: url }).catch(() => {
    window.open(url, '_blank');
  });
}

// History + Cookie persistence
const HISTORY_KEY = 'pricecheck_history';
const COOKIE_CN_KEY = 'pricecheck_cookie_cn';
const COOKIE_INTL_KEY = 'pricecheck_cookie_intl';

function loadHistory() {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (raw) searchHistory.value = JSON.parse(raw);
    const cn = localStorage.getItem(COOKIE_CN_KEY);
    if (cn) sessionCookieCn.value = cn;
    const intl = localStorage.getItem(COOKIE_INTL_KEY);
    if (intl) sessionCookieIntl.value = intl;
  } catch { /* ignore */ }
}

function saveCookie() {
  try {
    if (sessionCookieCn.value) {
      localStorage.setItem(COOKIE_CN_KEY, sessionCookieCn.value);
    } else {
      localStorage.removeItem(COOKIE_CN_KEY);
    }
    if (sessionCookieIntl.value) {
      localStorage.setItem(COOKIE_INTL_KEY, sessionCookieIntl.value);
    } else {
      localStorage.removeItem(COOKIE_INTL_KEY);
    }
  } catch { /* ignore */ }
}

function saveHistory() {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(searchHistory.value.slice(0, 20)));
  } catch { /* ignore */ }
}

function clearHistory() {
  searchHistory.value = [];
  localStorage.removeItem(HISTORY_KEY);
}

function clearSearch() {
  manualText.value = '';
  parsedItem.value = null;
  searchResult.value = null;
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
}

const rarityColors: Record<string, string> = {
  '普通': '#c8c8c8', 'Normal': '#c8c8c8',
  '魔法': '#8888ff', 'Magic': '#8888ff',
  '稀有': '#ffff77', 'Rare': '#ffff77',
  '传奇': '#af6025', 'Unique': '#af6025',
  '通货': '#aa9e82', 'Currency': '#aa9e82',
};
</script>

<template>
  <div class="pricecheck-page">
    <!-- Toast notification — fixed at top -->
    <transition name="toast-fade">
      <div class="toast-bar" :class="statusType" v-if="statusMsg">
        <span class="status-dot"></span>
        <span class="toast-text">{{ statusMsg }}</span>
      </div>
    </transition>

    <div class="pc-layout">
      <!-- Left: Input + Parsed Item -->
      <div class="pc-left">
        <!-- Manual Input -->
        <div class="glass-panel input-panel">
          <div class="panel-header">
            <span class="panel-title">物品数据</span>
            <span class="panel-hint">游戏中按 <kbd>Ctrl+C</kbd> 复制物品后在此粘贴，或直接按 <kbd>Ctrl+D</kbd> 自动查价</span>
          </div>
          <textarea
            v-model="manualText"
            class="item-input"
            placeholder="在此粘贴 PoE 物品数据..."
            rows="5"
          ></textarea>
          <div class="input-actions">
            <button class="glass-button" @click="clearSearch">清空</button>
          </div>
        </div>

        <!-- Parsed Item -->
        <div v-if="parsedItem" class="glass-panel result-panel">
          <div class="panel-header">
            <span class="panel-title">解析结果</span>
          </div>

          <!-- Item card -->
          <div class="item-card">
            <div class="item-head">
              <div class="item-rarity-name">
                <span class="item-rarity" :style="{ color: rarityColors[parsedItem.rarity] || '#c8c8c8' }">
                  {{ parsedItem.rarity }}
                </span>
                <span class="item-name" :style="{ color: rarityColors[parsedItem.rarity] || '#c8c8c8' }">
                  {{ parsedItem.name }}
                </span>
              </div>
              <span class="item-type">{{ parsedItem.baseType }}</span>
            </div>

            <!-- Item meta row -->
            <div class="item-meta-row" v-if="parsedItem.itemLevel || parsedItem.sockets || Object.keys(parsedItem.requirements).length">
              <span v-if="parsedItem.itemLevel" class="meta-chip">
                <span class="chip-label">物等</span> {{ parsedItem.itemLevel }}
              </span>
              <span v-if="parsedItem.sockets" class="meta-chip">
                <span class="chip-label">孔</span> {{ parsedItem.sockets.count }}<template v-if="parsedItem.sockets.links > 1">&nbsp;L{{ parsedItem.sockets.links }}</template>
              </span>
              <span v-for="(val, key) in parsedItem.requirements" :key="'req-' + key" class="meta-chip">
                <span class="chip-label">{{ key }}</span> {{ val }}
              </span>
            </div>

            <!-- Stats -->
            <div v-if="Object.keys(parsedItem.stats).length > 0" class="item-stats">
              <div v-for="(val, key) in parsedItem.stats" :key="key" class="stat-row">
                <span class="stat-key">{{ key }}</span>
                <span class="stat-val">{{ val }}</span>
              </div>
            </div>

            <!-- Mods -->
            <div v-if="parsedItem.mods.length > 0" class="item-mods">
              <div v-for="(mod, idx) in parsedItem.mods" :key="idx" class="mod-row">
                <span class="mod-type-pill" :class="mod.type.toLowerCase()">{{ mod.type }}</span>
                <span class="mod-text">{{ mod.text }}</span>
                <span v-if="mod.tier" class="mod-tier">T{{ mod.tier }}</span>
              </div>
            </div>
          </div>

          <!-- Trade actions -->
          <div class="trade-actions">
            <button class="glass-button primary search-btn" :disabled="isSearching" @click="searchTrade">
              <span v-if="isSearching" class="spinner"></span>
              <span v-else>搜索市集</span>
            </button>
          </div>

          <!-- Search result -->
          <div v-if="searchResult" class="search-result">
            <div class="result-info">
              <span v-if="searchResult.total !== undefined">
                找到 <strong>{{ searchResult.total }}</strong> 个结果
              </span>
            </div>
            <button class="glass-button primary" @click="openTradeUrl(searchResult.url)">
              打开市集页面
            </button>
          </div>
        </div>

        <!-- Empty state -->
        <div v-else class="glass-panel empty-state">
          <div class="empty-icon">📋</div>
          <p>粘贴物品数据或按 <kbd>Ctrl+D</kbd> 开始查价</p>
        </div>
      </div>

      <!-- Right sidebar -->
      <div class="pc-right">
        <!-- Server toggle -->
        <div class="glass-panel sidebar-section">
          <div class="panel-header">
            <span class="panel-title">服务器</span>
          </div>
          <div class="server-toggle">
            <button
              class="toggle-btn"
              :class="{ active: server === 'cn' }"
              @click="server = 'cn'"
            >
              <span class="toggle-label">国服</span>
              <span class="toggle-url">poe.game.qq.com</span>
            </button>
            <button
              class="toggle-btn"
              :class="{ active: server === 'intl' }"
              @click="server = 'intl'"
            >
              <span class="toggle-label">国际服</span>
              <span class="toggle-url">pathofexile.com</span>
            </button>
          </div>
        </div>

        <!-- League selector -->
        <div class="glass-panel sidebar-section">
          <div class="panel-header">
            <span class="panel-title">赛季</span>
          </div>
          <select v-model="league" class="league-select">
            <option v-for="l in leagueOptions" :key="l" :value="l">{{ l }}</option>
          </select>
        </div>

        <!-- Session Cookie -->
        <div class="glass-panel sidebar-section">
          <div class="panel-header">
            <span class="panel-title">Session Cookie</span>
          </div>
          <div class="cookie-section">
            <p class="cookie-hint">
              登录市集网站后，F12 → Network → 复制完整 Cookie 字符串
            </p>
            <input
              v-if="server === 'cn'"
              v-model="sessionCookieCn"
              @change="saveCookie"
              class="cookie-input"
              placeholder="粘贴国服 Cookie..."
              type="text"
            />
            <input
              v-else
              v-model="sessionCookieIntl"
              @change="saveCookie"
              class="cookie-input"
              placeholder="粘贴国际服 Cookie..."
              type="text"
            />
          </div>
        </div>

        <!-- History -->
        <div class="glass-panel sidebar-section history-section">
          <div class="panel-header">
            <span class="panel-title">查价历史</span>
            <button v-if="searchHistory.length > 0" class="btn-text" @click="clearHistory">清空</button>
          </div>
          <div v-if="searchHistory.length === 0" class="history-empty">
            <span>暂无记录</span>
          </div>
          <div v-else class="history-list">
            <div
              v-for="(h, idx) in searchHistory"
              :key="idx"
              class="history-item"
            >
              <div class="history-top">
                <span class="history-name">{{ h.name }}</span>
                <span class="history-time">{{ formatTime(h.time) }}</span>
              </div>
              <div class="history-sub">
                <span>{{ h.type }}</span>
                <span class="history-server">{{ serverLabel[h.server] || h.server }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Help -->
        <div class="glass-panel sidebar-section">
          <div class="panel-header">
            <span class="panel-title">使用说明</span>
          </div>
          <div class="help-content">
            <p><kbd>Ctrl+D</kbd> 快速查价</p>
            <p><kbd>Ctrl+Alt+D</kbd> 固定查价窗口</p>
            <p>也可 <kbd>Ctrl+C</kbd> 复制后粘贴</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pricecheck-page {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  color: #eee;
  position: relative;
}

/* Toast notification — fixed at top center */
.toast-bar {
  position: absolute;
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 999;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 24px;
  border-radius: 8px;
  font-size: 13px;
  max-width: 600px;
  white-space: normal;
  box-shadow: 0 8px 24px rgba(0,0,0,0.4);
}
.toast-bar.info {
  background: rgba(64, 158, 255, 0.15);
  border: 1px solid rgba(64, 158, 255, 0.35);
  color: #a0cfff;
  backdrop-filter: blur(12px);
}
.toast-bar.success {
  background: rgba(103, 194, 58, 0.15);
  border: 1px solid rgba(103, 194, 58, 0.35);
  color: #b6e68b;
  backdrop-filter: blur(12px);
}
.toast-bar.error {
  background: rgba(245, 108, 108, 0.15);
  border: 1px solid rgba(245, 108, 108, 0.35);
  color: #f9a3a3;
  backdrop-filter: blur(12px);
}
.toast-text {
  line-height: 1.5;
}
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: all 0.3s ease;
}
.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-12px);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.7;
  flex-shrink: 0;
}

/* Layout */
.pc-layout {
  display: flex;
  height: 100%;
  gap: 16px;
  padding: 12px 16px 16px 16px;
  overflow: hidden;
}

.pc-left {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-right: 4px;
}

.pc-right {
  width: 240px;
  flex-shrink: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* Glass panels — matching FilterPage */
.glass-panel {
  background: rgba(30, 30, 30, 0.6);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
}

.panel-header {
  padding: 10px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: #fff;
}

.panel-hint {
  font-size: 11px;
  color: #888;
}

.panel-hint kbd {
  background: rgba(255,255,255,0.08);
  padding: 1px 5px;
  border-radius: 3px;
  border: 1px solid rgba(255,255,255,0.12);
  font-size: 10px;
  color: #ccc;
}

/* Input */
.input-panel {
  flex-shrink: 0;
}

.item-input {
  width: 100%;
  background: rgba(0, 0, 0, 0.3);
  border: none;
  color: #d4d4d4;
  font-family: 'Consolas', 'Courier New', monospace;
  font-size: 12px;
  padding: 12px 14px;
  resize: vertical;
  box-sizing: border-box;
  line-height: 1.6;
  outline: none;
}
.item-input::placeholder {
  color: #555;
}

.input-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  padding: 8px 14px 12px;
}

/* Buttons — matching FilterPage */
.glass-button {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #eee;
  padding: 6px 14px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}
.glass-button:hover {
  background: rgba(255, 255, 255, 0.18);
}
.glass-button.primary {
  background: rgba(64, 158, 255, 0.3);
  border-color: rgba(64, 158, 255, 0.5);
}
.glass-button.primary:hover {
  background: rgba(64, 158, 255, 0.4);
}
.glass-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-text {
  background: none;
  border: none;
  color: #999;
  cursor: pointer;
  font-size: 11px;
  padding: 0;
}
.btn-text:hover {
  color: #ccc;
}

/* Result panel */
.result-panel {
  flex: 1;
}

/* Item card */
.item-card {
  padding: 14px;
}

.item-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.item-rarity-name {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.item-rarity {
  font-size: 13px;
  opacity: 0.85;
}
.item-name {
  font-size: 20px;
  font-weight: 700;
  line-height: 1.2;
}
.item-type {
  font-size: 13px;
  color: #aaa;
}

/* Meta chips */
.item-meta-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}
.meta-chip {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  color: #cfd8dc;
  font-size: 12px;
}
.chip-label {
  color: #999;
  margin-right: 5px;
  font-size: 11px;
}
.meta-chip.note-chip {
  color: #cbb688;
  border-color: rgba(170, 158, 130, 0.25);
  background: rgba(170, 158, 130, 0.08);
}

/* Stats */
.item-stats {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 10px;
  margin-bottom: 10px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.stat-row {
  display: flex;
  justify-content: space-between;
  padding: 3px 0;
  font-size: 13px;
}
.stat-key {
  color: #7f7f7f;
}
.stat-val {
  color: #d4d4d4;
}

/* Mods */
.item-mods {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.mod-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}
.mod-type-pill {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.06);
  color: #999;
  flex-shrink: 0;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.mod-type-pill.prefix {
  background: rgba(136, 136, 255, 0.12);
  color: #b0b0ff;
  border: 1px solid rgba(136, 136, 255, 0.25);
}
.mod-type-pill.suffix {
  background: rgba(255, 136, 136, 0.12);
  color: #ffb0b0;
  border: 1px solid rgba(255, 136, 136, 0.25);
}
.mod-text {
  color: #ddd;
  flex: 1;
}
.mod-tier {
  font-size: 10px;
  color: #777;
  flex-shrink: 0;
  background: rgba(255,255,255,0.04);
  padding: 1px 6px;
  border-radius: 3px;
}

/* Trade actions */
.trade-actions {
  padding: 12px 14px;
  display: flex;
  gap: 10px;
}
.search-btn {
  padding: 8px 24px;
  font-size: 14px;
}

/* Search result */
.search-result {
  margin: 0 14px 14px;
  padding: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: rgba(64, 158, 255, 0.06);
  border: 1px solid rgba(64, 158, 255, 0.15);
  border-radius: 6px;
}
.result-info {
  font-size: 13px;
  color: #ccc;
}

/* Empty state */
.empty-state {
  align-items: center;
  justify-content: center;
  color: #666;
  flex: 1;
}
.empty-icon {
  font-size: 48px;
  opacity: 0.25;
  margin-bottom: 12px;
}
.empty-state p {
  font-size: 14px;
}
.empty-state kbd {
  background: rgba(255,255,255,0.06);
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.08);
  font-size: 13px;
  color: #ccc;
}

/* Sidebar sections */
.sidebar-section {
  flex-shrink: 0;
}

/* Server toggle */
.server-toggle {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.toggle-btn {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 12px 14px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.2);
  color: #999;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}
.toggle-btn:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.15);
  color: #ccc;
}
.toggle-btn.active {
  background: rgba(64, 158, 255, 0.12);
  border: 1px solid rgba(64, 158, 255, 0.35);
  color: #fff;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.08);
}
.toggle-label {
  font-size: 14px;
  font-weight: 600;
}
.toggle-btn.active .toggle-label {
  color: #fff;
}
.toggle-url {
  font-size: 10px;
  opacity: 0.5;
  margin-top: 2px;
}
.toggle-btn.active .toggle-url {
  opacity: 0.7;
  color: #a0cfff;
}

/* League select */
.league-select {
  margin: 10px 12px 14px;
  width: calc(100% - 24px);
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  color: #eee;
  padding: 7px 10px;
  font-size: 13px;
  outline: none;
  cursor: pointer;
}
.league-select:focus {
  border-color: rgba(64, 158, 255, 0.35);
}
.league-select option {
  background: #1e222a;
  color: #eee;
}

/* Cookie section */
.cookie-section {
  padding: 10px 12px 14px;
}
.cookie-hint {
  font-size: 11px;
  color: #777;
  margin: 0 0 8px;
  line-height: 1.5;
}
.cookie-input {
  width: 100%;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  color: #ccc;
  font-size: 11px;
  padding: 6px 8px;
  box-sizing: border-box;
  outline: none;
  font-family: 'Consolas', monospace;
}
.cookie-input:focus {
  border-color: rgba(64, 158, 255, 0.35);
}
.cookie-input::placeholder {
  color: #555;
}

/* History */
.history-section {
  flex: 1;
  min-height: 0;
}
.history-empty {
  text-align: center;
  color: #666;
  font-size: 13px;
  padding: 24px 0;
}
.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.history-item {
  padding: 8px 10px;
  border-radius: 4px;
  transition: background 0.15s;
}
.history-item:hover {
  background: rgba(255, 255, 255, 0.05);
}
.history-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.history-name {
  font-size: 13px;
  color: #ddd;
}
.history-time {
  font-size: 10px;
  color: #666;
}
.history-sub {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: #777;
  margin-top: 2px;
}
.history-server {
  color: #888;
  padding: 0 4px;
  border-radius: 2px;
  background: rgba(255,255,255,0.03);
}

/* Help */
.help-content {
  padding: 12px 14px;
  font-size: 12px;
  color: #999;
  line-height: 2.2;
}
.help-content kbd {
  background: rgba(255,255,255,0.06);
  padding: 1px 6px;
  border-radius: 3px;
  border: 1px solid rgba(255,255,255,0.08);
  font-size: 11px;
  color: #ccc;
}

/* Spinner */
.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.2);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Scrollbar */
.pc-left::-webkit-scrollbar,
.pc-right::-webkit-scrollbar,
.history-list::-webkit-scrollbar {
  width: 6px;
}
.pc-left::-webkit-scrollbar-thumb,
.pc-right::-webkit-scrollbar-thumb,
.history-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 3px;
}
</style>
