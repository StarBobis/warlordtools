# Warlord Tools (藏身处战神工具箱)

<!-- Badges Section -->
<div align="center">

[![License](https://img.shields.io/github/license/StarBobis/warlordtools?style=flat-square)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools/network)
[![GitHub issues](https://img.shields.io/github/issues/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools/issues)
[![GitHub pull requests](https://img.shields.io/github/issues-pr/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools/pulls)
<br/>
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools/releases)
[![GitHub all releases](https://img.shields.io/github/downloads/StarBobis/warlordtools/total?style=flat-square)](https://github.com/StarBobis/warlordtools/releases)
[![GitHub repo size](https://img.shields.io/github/repo-size/StarBobis/warlordtools?style=flat-square)](https://github.com/StarBobis/warlordtools)

</div>

<p align="center">
  <a href="https://github.com/StarBobis/warlordtools">
    <img src="https://img.shields.io/badge/Powered%20by-Tauri%20v2-blue?style=for-the-badge&logo=tauri" alt="Tauri v2">
  </a>
  <a href="https://vuejs.org/">
    <img src="https://img.shields.io/badge/Frontend-Vue%203-4FC08D?style=for-the-badge&logo=vue.js&logoColor=white" alt="Vue 3">
  </a>
</p>

## Star History

<div align="center">
 <a href="https://star-history.com/#StarBobis/warlordtools&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=StarBobis/warlordtools&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=StarBobis/warlordtools&type=Date" />
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=StarBobis/warlordtools&type=Date" />
  </picture>
 </a>
</div>

## 简介

专为流放之路2（Path of Exile2）玩家打造的轻量化全能助手工具。基于 **Tauri v2** + **Vue 3** 构建。

QQ交流群 977107224 (BUG反馈和改进建议)

## 功能特性

- **过滤器本地编辑**：无需上传网页，快速修改过滤器规则。
- **常用工具集成**：集成了一些常用工具网页

## 开发

### 环境要求

- Rust (用于 Tauri 后端)
- Node.js (推荐 v18+) & Bun/pnpm/npm (用于前端)

### 运行

```bash
# 安装依赖
bun install

# 启动开发服务器
bun tauri dev
```

### 构建排查

```bash
# 遇到构建问题尝试
cargo clean
bun tauri build
```

# 补充信息

- 制作自定义音效可以试试8bit效果生成器，测了效果还不错：https://sfxr.me/
- 过滤器规则参考：https://www.pathofexile.com/item-filter/about
