# IPC

无参数启动 `bar-lyrics` 即进入 IPC 模式。每个插件持有自己的后端实例，无额外常驻控制进程。协议采用 [JSON-RPC 2.0](https://www.jsonrpc.org/specification)，HTTP 只监听 `127.0.0.1` 的随机端口，使用每次启动生成的 256 位随机令牌。

## 建立连接

Noctalia / DMS 从 stdout 的 `ready` 通知获取连接信息：

```json
{"jsonrpc":"2.0","method":"ready","params":{"version":1,"pid":1234,"url":"http://127.0.0.1:12345/rpc","token":"..."}}
```

向该地址 POST JSON，设置 `Content-Type: application/json` 和 `Authorization: Bearer <token>`。不支持跨域、重定向或无令牌访问。请求上限 64 KiB，批次最多 64 项，保留请求 ID、标准错误码和无 ID 通知语义；纯通知返回 HTTP 204。令牌不要写入日志或命令行。

实例信息也保存在 `$XDG_RUNTIME_DIR/bar-lyrics/<PID>-<随机标识>.json`，目录权限 0700、文件权限 0600。进程正常退出会移除文件；被强制结束后可能留下记录，使用前需核实 PID 和接口是否仍然有效，不要选择其他 Bar 的实例。

## 配置

```json
{"jsonrpc":"2.0","id":1,"method":"config.update","params":{"subtitle":"romanization","max_chars":40,"offset_ms":0}}
```

配置增量合并并整体校验，成功后返回完整有效配置；错误返回 `-32602`，旧配置不变。`config.get` 可读取当前配置。显示选项直接更新，数据源地址变化只重建源监听，不退出后端进程。`output` 是流格式，只能在启动时设置。

字段包括 `source`、`source_endpoint`、`output`、`offset_ms`、`max_chars`、`alignment`、`subtitle`、`inactive_opacity`、`cover_dir`、`current_cover`、`waybar_signal`，取值及默认值见 [`Options`](../src/options.rs)。可选路径设为 `null` 或空字符串即可清除。配置仅保存在内存；持久设置由各平台负责。

Waybar 启动时从 stdin 读取一条无 ID 的 `config.update` 通知，设置 `output: "waybar"`，随后通过同一 HTTP 接口动态更新。它不向 stdout 写入 `ready` 或 RPC 响应，以免污染模块 JSON；连接信息从运行目录读取。修改启动通知文件不会自动改变已运行实例。

## 播放控制

```json
{"jsonrpc":"2.0","id":2,"method":"player.control","params":{"action":"toggle"}}
```

操作为 `play`、`pause`、`toggle`、`previous`、`next`。后端先使用当前配置源的 WS，再尝试同源 MPRIS；全部失败返回 `-32001`，Noctalia / DMS 最后调用平台通用媒体控制。远程地址失败时跳过本机同源 MPRIS。控制在独立于渲染循环的 IPC 线程串行执行，不为每次点击启动进程。

stdout 的 `frame` 通知承载歌词帧，结构为 `{"jsonrpc":"2.0","method":"frame","params":{...}}`。旧命令行入口仅为兼容与诊断保留，不提供热更新；新插件和 Waybar 示例均不通过它传递配置。
