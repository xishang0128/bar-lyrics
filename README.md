# Bar Lyrics

面向桌面 Bar 的单行同步歌词项目。目前提供 Noctalia v5 插件与 SPlayer Next 数据源；Rust 负责数据读取、同步和生成显示帧，适配层只桥接平台并渲染 UI。

![效果预览](assets/preview.gif)

## 功能

- 单行同步歌词、逐字高亮、翻译或音译副行与切换动画
- 长歌词适配和歌曲封面缓存
- 可扩展的数据源与 Bar 输出接口

## 安装

先在 SPlayer 中启用外部 API 和 WebSocket，然后执行：

```bash
cargo build --release --locked
install -Dm755 target/release/bar-lyrics plugin/bin/bar-lyrics
mkdir -p ~/.local/share/noctalia/plugins
cp -aT plugin ~/.local/share/noctalia/plugins/bar-lyrics
noctalia msg plugins enable xishang0128/bar-lyrics
```

在 Noctalia Bar 中添加 `xishang0128/bar-lyrics:lyrics` widget。默认 API 地址为 `http://127.0.0.1:14558`。持续运行依赖 `/ws` 的实时事件；HTTP 只用于初始状态和低频位置校准。

### Waybar

先安装二进制：

```bash
install -Dm755 target/release/bar-lyrics ~/.local/bin/bar-lyrics
```

配置、样式和左右对齐方式见 [`examples/waybar`](examples/waybar/README.md)。

## 目录

```text
plugin/          Noctalia 清单、service、widget、sidecar 与翻译
src/cover.rs     封面下载与缓存
src/model.rs     与数据源、输出平台无关的领域模型
src/options.rs   sidecar 启动参数
src/source/      播放器与歌词数据源适配
src/lyrics.rs    逐字时间轴、截断与显示分段
src/engine.rs    播放状态、切换动画与显示帧
src/output/      Bar 输出格式适配
src/main.rs      流式输出循环
```

## 开发验证

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
noctalia plugins lint plugin
./target/release/bar-lyrics --source splayer --output json --once
```
