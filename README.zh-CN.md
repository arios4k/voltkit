<div align="center">

<img src="docs/icon.png" width="128" alt="Voltkit 图标：一道闪电穿过一把扳手">

# Voltkit

[English](README.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · **简体中文**

**为你的 Linux 桌面带来强大工具。**
屏幕 OCR、二维码扫描、取色器、屏幕标尺、图片和视频压缩器、文本扩展、智能粘贴、保持唤醒、hosts 与环境变量编辑器、
“是什么在占用它？”查找工具、应用权限管理器、休眠问题排查、系统报告、快捷键侦探和 GPU 调节器，集于一个应用。

[![最新版本](https://img.shields.io/github/v/release/arios4k/voltkit?label=%E7%89%88%E6%9C%AC&color=7c3aed)](https://github.com/arios4k/voltkit/releases/latest)
[![CI](https://github.com/arios4k/voltkit/actions/workflows/ci.yml/badge.svg)](https://github.com/arios4k/voltkit/actions/workflows/ci.yml)
[![手册](https://img.shields.io/badge/%E6%89%8B%E5%86%8C-%E9%98%85%E8%AF%BB-2ea44f)](https://arios4k.github.io/voltkit/manual/)
[![许可证：MIT](https://img.shields.io/badge/%E8%AE%B8%E5%8F%AF%E8%AF%81-MIT-blue)](LICENSE)
[![使用 apt 安装](https://img.shields.io/badge/%E5%AE%89%E8%A3%85-apt-orange)](#install-apt)
[![在 Fedora 上安装](https://img.shields.io/badge/%E5%AE%89%E8%A3%85-Fedora-51a2da)](#install-fedora)
[![使用 Rust 构建](https://img.shields.io/badge/%E4%BD%BF%E7%94%A8-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![通过 PayPal 支持](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.11.0">
  <img src="docs/screenshots/color-light.png?v=0.11.0" alt="Voltkit 的取色器，显示 #7C3AED 及其 HEX、RGB 和 HSL 值以及 WCAG 对比度评级">
</picture>

</div>

> Voltkit 提供简体中文、英语、西班牙语、德语和法语，并会自动使用你系统的语言：应用、命令行和手册页（`man voltkit`）
> 都是如此。本页的截图为英文版。

## 一个应用，取代十八个

在 Linux 上要凑齐这些基础功能，往往得为每件事各找一个小工具，每个都有自己的外观、设置和快捷键。Voltkit 把它们
放进一个原生 GTK4 应用，共用一套快捷键：

| 以前你得用… | Voltkit 给你 | 快捷键 |
|---|---|---|
| 没有：根本没有对应的界面 | **应用权限**：更改或遗忘你对应用的截屏、摄像头、位置等请求所给出的回答 | — |
| 单独的图片压缩器，再加一堆 ffmpeg 咒语 | **资源压缩器**：拖入 PNG、JPEG、WebP 或 MP4 文件，即可删除元数据并缩小体积 | 拖放 |
| 单独的取色器 | **取色器**：可吸取任意像素，支持 HEX/RGB/HSL、WCAG 对比度检查和历史记录 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> |
| 用 `mv` 写的 shell 循环或单独的重命名工具 | **批量重命名**：一次重命名许多文件，支持查找替换、正则表达式和编号，带预览和撤销 | — |
| 单独的图片缩放工具 | **图片缩放**：一次缩放许多图片，保持比例，不改动原文件 | — |
| 单独的 GPU 调节工具，外加各厂商的实用程序 | **GPU 调节器**：监控 NVIDIA、AMD 和 Intel 显卡，并更改功耗上限、风扇与风扇曲线、目标温度、频率和电压，每次更改都会先预览，未经确认即撤销 | — |
| 在终端里编辑 `/etc/hosts` 和 `~/.profile` | **Hosts 与环境变量**：开关 hosts 条目，并管理会话的环境变量 | — |
| 用来保持屏幕常亮的 shell 扩展 | **保持唤醒**：让电脑在 30 分钟到 4 小时内，或直到你关闭为止，不休眠、不锁屏 | — |
| 单独的演示画笔 | **缩放与批注**：冻结屏幕，放大并用画笔、箭头、方框和文字在上面绘制 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |
| 手机上的扫码工具或二维码网站 | **二维码**：读取屏幕上任意位置的二维码，或把文字、链接和 Wi-Fi 信息生成二维码 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> |
| 单独的屏幕标尺，或截图加图片编辑器 | **屏幕标尺**：冻结屏幕，以像素为单位测量任何内容 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> |
| 猜测某个键盘快捷键为什么没反应 | **快捷键侦探**：按下任意快捷键，查看它归谁所有、找出冲突、挑选空闲组合并修改 | — |
| 处理 JSON、表格和大小写的在线转换器 | **智能粘贴**：粘贴转换后的剪贴板内容：整理好的 JSON、转为 Markdown 的表格、去除跟踪参数的链接、去掉断行的 PDF 文字，或复制的图片中的文字 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> |
| 一堆终端命令来回答“我有什么硬件？” | **系统报告**：用通俗的话解释你的硬件，包含对驱动和磁盘健康的检查与修复、温度、固件更新、内存升级、更新后有什么变化，以及可放心分享的报告 | — |
| 单独的文本扩展工具 | **文本扩展**：可搜索的片段选择器，外加可选的键入触发词，如 `;date` | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> |
| 单独的 OCR 应用 | **文字提取器**：在屏幕上的任意文字周围拖出方框，视频和图片中的也行，文字会直接进入剪贴板 | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> |
| 终端里的 `lsof`、`fuser` 和 `ss` | **是什么在占用它？**：查看哪些应用占用了文件、文件夹、磁盘或端口，并将其关闭 | — |
| 在终端里翻系统日志 | **为什么没有休眠？**：查看是什么让电脑保持唤醒、是什么把它唤醒，以及休眠时用了多少电量 | — |

## 手册

**要了解如何使用 Voltkit，请阅读 [Voltkit 手册](https://arios4k.github.io/voltkit/manual/)**（英文）。它逐步讲解每个工具，包括键盘快捷键、命令行用法、配置和故障排除。也可以在应用中按 **F1** 或从菜单中选择**帮助与手册**打开。

## 为什么选择 Voltkit

- **为 Wayland 而生。** 截屏、取色和粘贴都使用桌面自带的门户，因此没有 X11 的取巧手段，也不需要 shell 扩展。
- **默认保护隐私。** 一切都在本地运行，没有网络访问，也没有遥测。键入触发词只在内存中保留最后几个字符，绝不记录或
  存储你输入的内容。
- **小巧而原生。** 一个 4 MB 的可执行文件（1.4 MB 的安装包），GTK4 + libadwaita，支持浅色和深色模式。
- **可脚本化。** 每项功能同时也是一条命令（`voltkit ocr`、`voltkit color '#3584e4'`、`voltkit compress *.jpg`），
  并为它们都提供了手册页。
- **自我检查。** `voltkit doctor` 会准确告诉你缺少什么，以及安装它的命令。
- **签名的软件包。** 通过 apt 从 GPG 签名的软件源安装和更新。
- **使用你的语言。** 简体中文、英语、西班牙语、德语和法语，跟随你系统的语言。

Voltkit 在 GNOME（Zorin OS 18）上开发和测试。它使用标准的 XDG 门户，因此 KDE Plasma 和其他桌面应该也能运行；欢迎反馈。更多解答见手册中的[常见问题](https://arios4k.github.io/voltkit/manual/faq.html)。

<sub>截图在 Zorin OS 18 上使用 Voltkit 的浅色和深色配色拍摄。可用 `docs/screenshots/capture.sh` 重新生成
（或用 `docs/screenshots/capture.sh zh_CN` 生成中文版应用的截图；也支持 `es`、`de` 和 `fr`）。</sub>


<a id="install-apt"></a>

## 安装（Ubuntu、Debian 及其衍生版）

适用于 Ubuntu 24.04+、Debian 13+、Linux Mint 22、Zorin OS 18、Pop!_OS 24.04 及其他衍生版，64 位 Intel/AMD：

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

apt 会在安装 Voltkit 的同时安装所需工具（Tesseract、wl-clipboard、桌面门户），之后的更新通过 `sudo apt upgrade` 获得。

然后从应用列表中打开 Voltkit：它会自动设置后台服务和键盘快捷键。接下来该做什么，请参阅手册中的 [Install](https://arios4k.github.io/voltkit/manual/install.html) 和 [A tour of the app](https://arios4k.github.io/voltkit/manual/tour.html)。

<a id="install-fedora"></a>

## 安装（Fedora）

适用于 64 位 Intel/AMD 的 Fedora 43 及更高版本，来自 [COPR 软件源](https://copr.fedorainfracloud.org/coprs/arios/voltkit/)：

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

然后从应用列表中打开 Voltkit；它会以同样的方式完成自身设置。更新通过 `sudo dnf upgrade` 获得。

## 安装（从源码）

要求：Rust 1.85+、GTK ≥ 4.14、libadwaita ≥ 1.5。

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita
```

```bash
cargo install --path .     # 安装到 ~/.cargo/bin/voltkit
voltkit
```

请在设置快捷键或开机自启之前先安装可执行文件：两者都会记录该文件的路径。

要用图标把 Voltkit 添加到应用网格：

```bash
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
sed "s|^Exec=voltkit|Exec=$HOME/.cargo/bin/voltkit|" data/io.github.arios4k.Voltkit.desktop > ~/.local/share/applications/io.github.arios4k.Voltkit.desktop
```

## 翻译

Voltkit 已翻译为西班牙语、德语、法语和简体中文。文字保存在 [`po/`](po/) 中的普通 gettext 文件里，并已内置到可执行文件中，
所以无需额外安装任何东西。要添加语言或修正翻译，请参阅 [`po/README.md`](po/README.md)。

## 项目结构

```
src/
  main.rs            命令行入口 (clap) → 不带子命令时进入图形界面
  i18n.rs            翻译：内置的 .po 目录、系统语言
  app.rs             GTK4 / libadwaita 控制中心
  daemon.rs          后台服务：D-Bus API、门户快捷键、开机自启
  actions.rs         快捷键操作（OCR、取色）及通知
  shortcuts.rs       快捷键转换 + GNOME 快捷键后备方案
  picker.rs          片段选择器弹窗
  snippets_ui.rs     文本扩展页面：片段列表和编辑器
  qr_ui.rs           二维码页面
  ruler.rs           屏幕标尺（冻结截图的覆盖层）
  awake.rs           保持唤醒（Inhibit 门户，由守护进程持有）
  awake_ui.rs        保持唤醒页面
  system_ui.rs       Hosts 与环境变量页面
  usage_ui.rs        “是什么在占用它？”页面
  permissions_ui.rs  应用权限页面
  sleep_ui.rs        “为什么没有休眠？”页面
  gpu_ui.rs          GPU 调节器页面：读数、支持情况报告、控制项
  gpu_cli.rs         `voltkit gpu`、root 辅助程序和后台服务
  sysreport_ui.rs    系统报告页面
  detective_ui.rs    快捷键侦探页面
  smartpaste_ui.rs   智能粘贴窗口和页面
  expander.rs        键入触发词（evdev 读取键盘，uinput 虚拟键盘）
  paste.rs           粘贴到当前焦点所在的应用，并恢复剪贴板
  portal.rs          XDG 门户调用（截图、PickColor）和通知
  config.rs          带默认值的 TOML 配置
  system.rs          Wayland/X11 辅助：剪贴板、工具发现、进程运行器
  modules/
    ocr.rs           区域截取 + Tesseract
    color.rs         解析、HSL、WCAG 亮度与对比度
    compress.rs      格式检测 + 各格式的压缩流程
    snippets.rs      片段模板引擎
    qr.rs            二维码读取与生成
    hosts.rs         解析 /etc/hosts，并保留所有它未编辑的内容
    envvars.rs       会话环境变量 (environment.d)
    usage.rs         查找并关闭占用某个路径或端口的进程 (/proc)
    permissions.rs   桌面门户的权限存储
    sleep.rs         休眠阻止项、休眠历史（日志、UPower）和唤醒设备
    gpu.rs           GPU 检测、传感器以及每块显卡支持什么（NVIDIA、AMD、Intel）
    gpu_tune.rs      经过验证、具事务性的 GPU 更改、风扇曲线、启动保护
    nvml.rs          NVIDIA 管理库的绑定
    sysreport.rs     硬件、检查项、磁盘健康、传感器、固件、快照、SMBIOS 内存
    keybindings.rs   所有 GSettings 快捷键：所有者、冲突、空闲组合、更改
    smartpaste.rs    智能粘贴的文本转换
  manpages.rs        手册页生成器（clap_mangen + 手写章节）
  screenshots.rs     生成 README 截图（隐藏的 `screenshots` 命令）
po/                  翻译（西班牙语、德语、法语、简体中文）以及用于更新它们的 update.py
man/                 生成的手册页（man/es、man/de、man/fr、man/zh_CN：已翻译）
packaging/apt/        APT 软件源构建器及其落地页
docs/screenshots/    README 截图以及用于重新生成它们的 capture.sh
docs/ARCHITECTURE.md 设计说明和路线图
```

## 参与贡献

```bash
cargo test
cargo clippy
python3 po/update.py --check   # 翻译是否最新且完整
```

修改命令或其帮助文本后，请用 `cargo run -- manpages man` 重新生成 [`man/`](man/) 中的手册页；在此之前 `cargo test` 会失败。

D-Bus API 和路线图请参阅 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。接下来的重头戏是 设置界面。尤其欢迎
错误报告以及在其他桌面上的测试。

## 发布

1. 在 `Cargo.toml` 中提高 `version`，运行 `cargo run -- manpages man` 和 `python3 po/update.py`，然后提交。
2. 打标签并推送：`git tag v0.2.0 && git push origin v0.2.0`。

[发布工作流](.github/workflows/release.yml)会构建 `.deb`，用它发布一个 GitHub Release，并在 GitHub Pages 上重建已签名的
APT 软件源。它使用 `APT_SIGNING_KEY` 机密变量中的密钥签名（指纹 `65E8 4B8D 114C 6147 9CCA 4DDC EC0F 6B66 E480 2360`）。
要在本地构建软件源：`APT_KEY_ID=<指纹> packaging/apt/build-repo.sh target/debian site`。

## 作者

由 **Alexis Rios-Negron** 创建。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.11.0">
  <img src="docs/screenshots/about-light.png?v=0.11.0" width="640" alt="Voltkit 的“关于”窗口，显示应用图标、版本和作者 Alexis Rios-Negron">
</picture>

## 支持 Voltkit

如果 Voltkit 对你有帮助，欢迎[捐赠任意金额](https://paypal.me/AlexisRios)，帮助项目持续运行和开发。每一份支持都值得感谢。

我们希望 Voltkit 始终免费，让每个人都能使用。感谢你的使用，也感谢你给予的任何支持！

## 许可证

MIT © Alexis Rios-Negron
