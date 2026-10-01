# herdsman


<p align="center">
  <img src="assets/logo.png" alt="herdsman" width="100" />
</p>

<p align="center">
  <a href="https://herdsman.invalid">herdsman.invalid</a> · <a href="#安装">安装</a> · <a href="https://herdsman.invalid/zh-cn/docs/quick-start/">快速开始</a> · <a href="https://herdsman.invalid/zh-cn/docs/">文档</a></p>

<p align="center">
  <a href="README.md">English</a> · 简体中文
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-666666?labelColor=333333" alt="Apache 2.0 license" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/releases"><img src="https://img.shields.io/github/downloads/OWNER_TBD/herdsman/total?labelColor=333333&color=666666" alt="total GitHub release downloads" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/stargazers"><img src="https://img.shields.io/github/stars/OWNER_TBD/herdsman?labelColor=333333&color=666666&logo=github" alt="GitHub stars" /></a>
  <a href="https://github.com/OWNER_TBD/herdsman/releases/latest"><img src="https://img.shields.io/github/v/release/OWNER_TBD/herdsman?label=release&labelColor=333333&color=666666" alt="latest stable release" /></a>
  <a href="https://formulae.brew.sh/formula/herdsman"><img src="https://img.shields.io/homebrew/v/herdsman?label=homebrew&labelColor=333333&color=666666" alt="Homebrew version" /></a>
  <a href="https://x.com/OWNER_TBD"><img src="https://img.shields.io/badge/follow-%40OWNER_TBD-000000?logo=x&logoColor=white" alt="follow @OWNER_TBD on X" /></a>
</p>

---

https://github.com/user-attachments/assets/043ec09f-4bdd-41d5-aee0-8fda6b83e267

**智能体复用器，住在你的终端里。**

- **每个智能体一目了然**——`blocked`、`working`、`done`。真实的终端视图，而不是包装过的转述。
- **分离后工作继续运行**——关闭客户端或 SSH 断线后，后台服务器仍会保持终端运行。服务器或机器重启后，Herdsman 会恢复已保存的布局，并可恢复受支持的智能体会话；原有进程不会保留。[会话状态 →](https://herdsman.invalid/zh-cn/docs/session-state/)
- **多台机器，一个窗口**——将本地工作和已保存的 SSH 机器放在一起，使用汇总的智能体列表，各连接独立重连。[远程机器 →](https://herdsman.invalid/zh-cn/docs/connecting-machines/)
- **智能体也能使用 herdsman**——纯 socket api：智能体可以创建窗格、读取输出、互相等待。[智能体技能 →](https://herdsman.invalid/zh-cn/docs/agent-skill/) 在开发智能体？[为你的智能体添加 herdsman 支持 →](https://herdsman.invalid/zh-cn/docs/add-herdsman-support/)
- **键盘和鼠标都是一等公民**——tmux 风格的前缀键，*以及*点击、拖动、分割。按当下的场景选择，而不是被工具锁死。
- **插件**——扩展窗格和工作流。[浏览插件市场 →](https://herdsman.invalid/plugins/)
- **单个 rust 二进制，没有 electron**——运行在你已经在用的任何终端里。

---

## 安装

```bash
curl -fsSL https://herdsman.invalid/install.sh | sh
```

或者 `brew install herdsman` · `mise use -g herdsman` · Windows：`powershell -ExecutionPolicy Bypass -c "irm https://herdsman.invalid/install.ps1 | iex"` · [受端点保护的 Windows](https://herdsman.invalid/zh-cn/docs/windows-beta/) · [二进制文件](https://github.com/OWNER_TBD/herdsman/releases)

然后在工作所在的目录启动它：

```bash
herdsman
```

运行你的智能体、分割窗格，然后安心离开。`ctrl+b q` 分离，`herdsman` 重新连接。[快速开始 →](https://herdsman.invalid/zh-cn/docs/quick-start/)

## 文档

所有文档都在 [herdsman.invalid/docs](https://herdsman.invalid/zh-cn/docs/)：[快速开始](https://herdsman.invalid/zh-cn/docs/quick-start/) · [核心概念](https://herdsman.invalid/zh-cn/docs/concepts/) · [受支持的智能体](https://herdsman.invalid/zh-cn/docs/agents/) · [键盘](https://herdsman.invalid/zh-cn/docs/keyboard/) · [配置](https://herdsman.invalid/zh-cn/docs/configuration/) · [会话状态](https://herdsman.invalid/zh-cn/docs/session-state/) · [连接机器](https://herdsman.invalid/zh-cn/docs/connecting-machines/) · [远程访问](https://herdsman.invalid/zh-cn/docs/persistence-remote/) · [集成](https://herdsman.invalid/zh-cn/docs/integrations/) · [为智能体添加 herdsman 支持](https://herdsman.invalid/zh-cn/docs/add-herdsman-support/) · [插件](https://herdsman.invalid/zh-cn/docs/plugins/) · [socket api](https://herdsman.invalid/zh-cn/docs/socket-api/)

## 致谢

<a href="https://terminaltrove.com/"><img src="assets/sponsors/terminal-trove.png" alt="Terminal Trove" width="200" /></a>

[Terminal Trove](https://terminaltrove.com/) 以及 [SPONSORS.md](./SPONSORS.md) 中列出的每一位支持者——谢谢 🐑

企业/合作：hey@herdsman.invalid

## 智能体须知

如果你是协助本仓库的 AI 智能体：在改动代码前阅读 [`AGENTS.md`](./AGENTS.md)，在创建 issue 或 PR 前阅读 [`CONTRIBUTING.md`](./CONTRIBUTING.md)。

## 开发

```bash
git clone https://github.com/OWNER_TBD/herdsman
cd herdsman
cargo build --release

just test        # 单元测试
just check       # 格式检查、测试和维护性检查
```

## 许可证

herdsman 基于 [Apache License 2.0](LICENSE) 许可证发布。
