# Waybar 配置

将 [`config.jsonc`](config.jsonc) 合并到 Waybar 配置，并按需合并 [`style.css`](style.css)。`custom/bar-lyrics` 是持续输出模块，不要给它设置 `interval`。使用前需在 SPlayer 中同时启用外部 API 与 WebSocket。

将启动通知安装到示例使用的位置：

```bash
install -Dm644 waybar/initial.json ~/.config/bar-lyrics/waybar.json
```

二进制不接收配置参数，启动时从 stdin 读取这条 `config.update` 通知；运行中也可通过 [IPC](../docs/ipc.md) 修改配置或控制播放，无需重启 Waybar。实例地址记录在 `$XDG_RUNTIME_DIR/bar-lyrics/<PID>-<随机标识>.json`，stdout 只输出 Waybar 格式。

## 封面与歌词对齐

默认示例将封面放在歌词右侧，胶囊宽度跟随歌词变化：

```jsonc
"modules-center": ["custom/bar-lyrics", "image#bar-lyrics-cover"]
```

如果希望切换歌词时封面位置不动，可给歌词分配固定宽度。封面在右侧时，让歌词
靠右对齐：

```jsonc
"custom/bar-lyrics": {
  "min-length": 32,
  "max-length": 32,
  "align": 1.0,
  "justify": "right"
}
```

封面在左侧时，交换模块顺序并让歌词靠左对齐：

```jsonc
"modules-center": ["image#bar-lyrics-cover", "custom/bar-lyrics"],
"custom/bar-lyrics": {
  "min-length": 32,
  "max-length": 32,
  "align": 0.0,
  "justify": "left"
}
```

同时将样式中的封面间距改到右侧：

```css
#image.bar-lyrics-cover {
  margin-left: 0;
  margin-right: 6px;
}
```

固定宽度会让胶囊不再跟随歌词长度；不需要固定封面时省略 `min-length` 和
`max-length` 即可。启动通知中的 `max_chars` 控制 Rust 输出的最大字符数；`alignment: "start"`
保留普通逐行歌词和歌曲名的开头，`alignment: "end"` 保留结尾。逐字歌词过长时会
自动围绕当前进度滚动。

## 其他说明

- 输出提供 `playing`、`paused`、`lyrics`、`fallback` 和 `hidden` CSS class。
- 逐字歌词使用 Pango markup 高亮，换行采用上移淡出、下方淡入动画。
- `subtitle` 可设为 `auto`、`translation`、`romanization` 或 `hidden`。
- 封面由 Rust 原子更新，Waybar 通过实时信号刷新，不会定时轮询或启动封面进程。
- 示例使用 `/run/user/1000`；UID 不同时需同步修改 `initial.json` 中的封面路径和 `config.jsonc` 中的 `path`。
- 信号 8 被占用时，需同步修改 `waybar_signal` 和 `signal`。
