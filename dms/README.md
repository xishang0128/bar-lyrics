# DMS

DankMaterialShell 的 DankBar 插件，支持逐字进度、翻译/音译、上下切换动画和封面。一个后台进程供所有 Bar 实例共用，配置与控制通过 IPC 传递，设置可实时更新；目前支持横向 Bar。

```bash
cargo build --release --locked
install -Dm755 target/release/bar-lyrics dms/bin/bar-lyrics
mkdir -p ~/.config/DankMaterialShell/plugins
cp -aT dms ~/.config/DankMaterialShell/plugins/barLyrics
```

在 DMS 设置中扫描并启用 **Bar Lyrics**，再添加到 DankBar。字号、副行、封面位置、对齐和最大宽度可在插件设置中调整。左键打开媒体控制器，右键切换播放状态，滚轮向上/向下切换上一首/下一首。SPlayer 需启用外部 API、WebSocket 和系统媒体控制。
