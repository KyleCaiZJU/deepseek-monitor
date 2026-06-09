# DeepSeek Monitor

常驻 Windows 系统托盘的小工具，实时监控 DeepSeek API 账户余额、消费金额、Token 用量和缓存命中率。

![面板预览](screenshots/panel.png)
![面板预览](screenshots/panel2.png)
![设置页面](screenshots/settings.png)

## 功能

- 系统托盘常驻，左键弹出毛玻璃面板
- 账户余额实时显示（红色/绿色在线状态灯）
- 当日/本月消费统计
- 缓存命中率（综合 + 按模型 + 按来源）
- 近 7 天 Token 消耗趋势图
- 模型用量明细（V4 Pro / V4 Flash / R1）
- 开机自启
- CSV 自动导入（下载目录监控）

## 系统要求

- Windows 10/11 x64
- 无需额外运行时（MSI 安装包自带 WebView2）

## 安装

1. 从 Release 页面下载 `deepseek-monitor_0.1.0_x64_en-US.msi`
2. 双击安装，一路下一步即可
3. 安装完成后，Win 键搜索 **DeepSeek** 启动

## 初始设置

启动后右键托盘图标选择"设置"（或左键弹出面板后点齿轮图标），填入两项配置：

### 1. API Key

前往 [platform.deepseek.com](https://platform.deepseek.com) 登录，点击左侧 **API Keys**，创建一个 Key（如 `monitor`），复制 `sk-` 开头的字符串。

**用途说明**：API Key 用于调用 `/user/balance` 接口查询**账户总余额**。注意：余额是**账户级别**（account-level），显示的是该账户下所有 Key 的合计余额，不是单个 Key 的余额。本工具**不是分 Key 追踪工具**——它是一个账户级别的监控面板。

### 2. Platform Token

Platform Token 用于从 DeepSeek 开放平台拉取聚合用量数据（消费记录、Token 统计等）。获取步骤：

1. 浏览器打开 [platform.deepseek.com](https://platform.deepseek.com) 并登录
2. 按 `F12` 打开开发者工具
3. 切换到 **Application**（应用程序）标签页
4. 左侧列表中找到 **Local Storage** -> `https://platform.deepseek.com`
5. 在右侧找到 `userToken` 这一项，复制它的值（格式类似 `eyJhbGciOi...` 的长字符串）

> 注意：Platform Token 会过期（通常数天到数周），如果数据拉取失败，重新按上述步骤获取即可。

保存后 App 会自动拉取数据并开始定时刷新。

## 验证是否正常工作

1. 确认托盘图标已出现（任务栏右下角，可能需要展开溢出区）
2. 左键点击托盘图标，面板应弹出在屏幕右下角
3. 面板顶部应显示"在线"（绿色圆点）和账户余额
4. 如果有历史充值/USE 记录，"当日消费"和"本月消费"应有对应金额
5. 右键托盘图标 -> "刷新"，数据应更新

如果一直显示"离线"（红色圆点），请检查 API Key 是否正确。

## 托盘操作

| 操作 | 效果 |
|------|------|
| 左键点击 | 弹出/隐藏面板 |
| 点击面板外任意位置 | 自动隐藏面板 |
| 右键 -> 刷新 | 手动拉取最新数据 |
| 右键 -> 设置 | 打开设置面板 |
| 右键 -> 开机自启 | 切换开机自动启动 |
| 右键 -> 退出 | 退出程序 |

## 数据来源

- **余额**：通过 DeepSeek 官方 API (`/user/balance`) 实时获取，刷新间隔可在设置中调整（默认 5 分钟）
- **用量/消费**：通过 Platform Token 从 DeepSeek 开放平台拉取聚合数据，每 30 分钟刷新一次
- 所有数据存储在本地 SQLite 数据库中（`%APPDATA%\deepseek-monitor\deepseek-monitor.db`），不上传任何第三方

## 常见问题

### Q: 提示"获取余额失败"或一直显示离线

- 检查 API Key 是否已正确填入（设置页面）
- 确认网络能访问 `api.deepseek.com`
- 确认 API Key 未过期、未删除

### Q: 消费/用量数据为空

- 确认 Platform Token 是否已正确填入
- Platform Token 可能已过期，重新从浏览器 Local Storage 复制
- 新创建的账户如果没有消费记录，数据自然为空

### Q: 面板弹出位置不对（多显示器）

v0.1.1 已修复：App 会根据点击托盘图标时的鼠标位置自动定位到正确的显示器。如果仍有问题，请升级到最新版本。

### Q: 开机自启不生效

- 确认已在右键菜单中勾选"开机自启"
- Windows 安全软件可能拦截了自启注册表项，请检查 Windows Defender 或杀毒软件设置

### Q: 如何卸载

通过 Windows 设置 -> 应用 -> 找到 DeepSeek Monitor -> 卸载。数据库文件需手动删除：`%APPDATA%\deepseek-monitor\`

## 隐私说明

- API Key 和 Platform Token 仅存储在本地，不会上传到任何第三方服务器
- 余额查询直连 `api.deepseek.com`
- 用量数据通过 Platform Token 从 `platform.deepseek.com` 获取
- 所有数据存储在本地 SQLite 中

## 给 Claude Code 用户的快速配置

如果你自己通过 Claude Code 帮别人安装配置，只需要让用户在浏览器执行以下 3 步，然后填入设置即可：

### 一条消息发过去

```
帮我配置 DeepSeek Monitor:

1. 打开 https://platform.deepseek.com 登录
2. 左侧 API Keys → 创建一个 Key，名称填 "monitor"，把 sk-... 发给我
3. 在 platform.deepseek.com 页面按 F12 → Application → Local Storage → 找到 userToken 的值，发给我

（如果看不到 Local Storage，确认已登录且页面是 platform.deepseek.com）
```

拿到两个值后，在 App 设置中填入，保存即可。整个过程不需要命令行操作。

## 技术栈

- Tauri 2 (Rust) + React 19 + TypeScript + Vite
- recharts (图表)
- SQLite (本地数据库)
- 毛玻璃 UI (纯 CSS)
