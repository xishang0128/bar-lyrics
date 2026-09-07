local FRAME_KEY = "bar_lyrics.frame"
local WIDGET_TYPE = "xishang0128/bar-lyrics:lyrics"

local function shellQuote(value)
  return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
end

local function publishError(message)
  noctalia.state.set(FRAME_KEY, {
    visible = true,
    error = message,
  })
end

local rpc = require("./ipc.luau")

local function displayOptions()
  local widgets = noctalia.getSetting("widget")
  local bars = noctalia.getSetting("bar")
  if type(widgets) ~= "table" or type(bars) ~= "table" then return "start", true end

  for _, bar in pairs(bars) do
    if type(bar) == "table" then
      for _, laneName in ipairs({ "start", "center", "end" }) do
        local lane = bar[laneName]
        if type(lane) == "table" then
          for index, name in ipairs(lane) do
            local settings = widgets[name]
            if type(settings) == "table" and settings.type == WIDGET_TYPE then
              local showCover = settings.show_cover ~= false
              if laneName ~= "center" then return laneName, showCover end

              for anchorIndex, anchorName in ipairs(lane) do
                local anchor = widgets[anchorName]
                if type(anchor) == "table" and anchor.anchor == true then
                  return index < anchorIndex and "end" or "start", showCover
                end
              end
              return index <= (#lane + 1) / 2 and "end" or "start", showCover
            end
          end
        end
      end
    end
  end
  return "start", true
end

function onConfigChanged()
  local alignment, showCover = displayOptions()
  local dataDir = noctalia.pluginDataDir()
  rpc.call("config.update", {
    source = "splayer",
    source_endpoint = noctalia.getConfig("base_url") or "http://127.0.0.1:14558",
    offset_ms = tonumber(noctalia.getConfig("offset_ms")) or 0,
    max_chars = tonumber(noctalia.getConfig("max_chars")) or 32,
    alignment = alignment,
    subtitle = noctalia.getConfig("subtitle") or "auto",
    cover_dir = showCover and dataDir and (dataDir .. "/covers") or "",
  }, function(error)
    if error then noctalia.log(error.message) end
  end)
end

function onIpc(event, command)
  if event ~= "control" or (command ~= "toggle" and command ~= "previous" and command ~= "next") then return end
  local function fallback()
    noctalia.runAsync("noctalia msg media " .. command)
  end
  if not rpc.address then fallback(); return end
  rpc.call("player.control", { action = command }, function(error)
    if error then
      noctalia.log(error.message)
      if error.code == -32001 or error.code == -32098 then fallback() end
    end
  end)
end

local function onLine(line)
  local message = noctalia.json.decode(line)
  if type(message) ~= "table" or message.jsonrpc ~= "2.0" then return end
  if message.method == "ready" then
    rpc.connect(message.params)
    onConfigChanged()
  elseif message.method == "frame" then
    noctalia.state.set(FRAME_KEY, message.params)
  end
end

noctalia.state.set(FRAME_KEY, { visible = false })
local pluginDir = noctalia.pluginDir()
local executable = pluginDir and (pluginDir .. "/bin/bar-lyrics") or ""
if executable == "" or not noctalia.fileExists(executable) then
  publishError(noctalia.tr("errors.binary_missing"))
elseif not noctalia.runStream("exec " .. shellQuote(executable), onLine) then
  publishError(noctalia.tr("errors.stream_failed"))
end
