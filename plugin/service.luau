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

local function onLine(line)
  local frame = noctalia.json.decode(line)
  if type(frame) == "table" then
    noctalia.state.set(FRAME_KEY, frame)
  end
end

local function displayOptions()
  local widgets = noctalia.getSetting("widget")
  local bars = noctalia.getSetting("bar")
  if type(widgets) ~= "table" or type(bars) ~= "table" then return "start", 32, true end

  for _, bar in pairs(bars) do
    if type(bar) == "table" then
      for _, laneName in ipairs({ "start", "center", "end" }) do
        local lane = bar[laneName]
        if type(lane) == "table" then
          for index, name in ipairs(lane) do
            local settings = widgets[name]
            if type(settings) == "table" and settings.type == WIDGET_TYPE then
              local maxChars = tonumber(settings.max_chars) or 32
              local showCover = settings.show_cover ~= false
              if laneName ~= "center" then return laneName, maxChars, showCover end

              for anchorIndex, anchorName in ipairs(lane) do
                local anchor = widgets[anchorName]
                if type(anchor) == "table" and anchor.anchor == true then
                  return index < anchorIndex and "end" or "start", maxChars, showCover
                end
              end
              return index <= (#lane + 1) / 2 and "end" or "start", maxChars, showCover
            end
          end
        end
      end
    end
  end
  return "start", 32, true
end

local function stream(executable)
  local baseUrl = noctalia.getConfig("base_url") or "http://127.0.0.1:14558"
  local pollInterval = tonumber(noctalia.getConfig("poll_interval_ms")) or 250
  local offset = tonumber(noctalia.getConfig("offset_ms")) or 0
  local alignment, maxChars, showCover = displayOptions()
  local command = shellQuote(executable)
    .. " --source splayer"
    .. " --source-endpoint " .. shellQuote(baseUrl)
    .. " --output json"
    .. " --poll-ms " .. tostring(pollInterval)
    .. " --offset-ms " .. tostring(offset)
    .. " --max-chars " .. tostring(maxChars)
    .. " --align " .. alignment
  if showCover then
    local dataDir = noctalia.pluginDataDir()
    if dataDir then command = command .. " --cover-dir " .. shellQuote(dataDir .. "/covers") end
  end

  if not noctalia.runStream(command, onLine) then
    publishError(noctalia.tr("errors.stream_failed"))
  end
end

noctalia.state.set(FRAME_KEY, { visible = false })
local pluginDir = noctalia.pluginDir()
local executable = pluginDir and (pluginDir .. "/bin/bar-lyrics") or ""
if executable == "" or not noctalia.fileExists(executable) then
  publishError(noctalia.tr("errors.binary_missing"))
else
  stream(executable)
end
