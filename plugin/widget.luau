local LINE_VIEWPORT_HEIGHT = 18
local FRAME_KEY = "bar_lyrics.frame"

local activeColor = noctalia.getConfig("active_color") or "on_surface"
local inactiveColor = noctalia.getConfig("inactive_color") or "on_surface_variant"
local inactiveOpacity = (tonumber(noctalia.getConfig("inactive_opacity")) or 45) / 100
local showWhenPaused = noctalia.getConfig("show_when_paused") ~= false
local showCover = noctalia.getConfig("show_cover") ~= false
local coverPosition = noctalia.getConfig("cover_position") or "right"
local coverSize = math.max(16, math.min(48, tonumber(noctalia.getConfig("cover_size")) or 24))
local coverRadius = math.max(0, math.min(coverSize / 2, tonumber(noctalia.getConfig("cover_radius")) or 5))
local showGlyph = noctalia.getConfig("show_glyph") == true
local glyph = noctalia.getConfig("glyph") or "music"
local fallback = noctalia.getConfig("fallback") or "track"
noctalia.getConfig("max_chars")

local function label(text, color, opacity, key)
  return ui.label({
    key = key,
    text = text,
    color = color,
    opacity = opacity,
    maxLines = 1,
    fontWeight = "bold",
    baseline = "textFixedHeight",
  })
end

local function lineNode(line)
  local nodes = {}
  for index, segment in ipairs(line.segments or {}) do
    local progress = math.max(0, math.min(1,
      tonumber(segment.progress) or (segment.active and 1 or 0)
    ))
    nodes[index] = label(
      segment.text or "",
      progress > 0 and activeColor or inactiveColor,
      inactiveOpacity + (1 - inactiveOpacity) * progress,
      "segment-" .. tostring(index)
    )
  end
  return ui.row({ key = "line-" .. tostring(line.key), gap = 0, align = "center" }, nodes)
end

local function lyricNode(frame)
  local transition = frame.transition
  if type(transition) ~= "table" then return lineNode(frame.line) end

  return ui.column({
    key = "line-transition",
    height = LINE_VIEWPORT_HEIGHT,
    gap = 0,
    align = frame.alignment or "start",
  }, {
    ui.row({
      key = "outgoing",
      height = LINE_VIEWPORT_HEIGHT * transition.outgoing_height_factor,
      align = "end",
      opacity = transition.outgoing_opacity,
    }, { lineNode(transition.outgoing) }),
    ui.row({
      key = "incoming",
      height = LINE_VIEWPORT_HEIGHT,
      align = "start",
      opacity = transition.incoming_opacity,
    }, { lineNode(frame.line) }),
  })
end

local function coverNode(path)
  if not showCover or type(path) ~= "string" or path == "" or not noctalia.fileExists(path) then
    return nil
  end
  return ui.image({
    path = path,
    width = coverSize,
    height = coverSize,
    radius = coverRadius,
    fit = "cover",
  })
end

local function render(frame)
  if type(frame) ~= "table" then
    barWidget.setVisible(false)
    return
  end
  if type(frame.error) == "string" then
    barWidget.render(ui.glyph({ name = "alert-circle", color = "error" }))
    barWidget.setTooltip(frame.error)
    barWidget.setVisible(true)
    return
  end
  if frame.visible ~= true
      or (frame.playing ~= true and not showWhenPaused)
      or (frame.line == nil and fallback == "hidden") then
    barWidget.setVisible(false)
    return
  end

  local children = {}
  local cover = coverNode(frame.cover_path)
  if showGlyph then children[#children + 1] = ui.glyph({ name = glyph, size = 14, color = activeColor }) end
  if cover and coverPosition ~= "right" then children[#children + 1] = cover end
  if frame.line then
    children[#children + 1] = lyricNode(frame)
  else
    children[#children + 1] = label(frame.fallback_text or "", inactiveColor, inactiveOpacity, "fallback")
  end
  if cover and coverPosition == "right" then children[#children + 1] = cover end

  local tooltip = frame.title or ""
  if frame.artist and frame.artist ~= "" then tooltip = tooltip .. " — " .. frame.artist end
  if frame.source and frame.source ~= "" then tooltip = tooltip .. "\n" .. frame.source end

  barWidget.render(ui.row({ gap = #children > 1 and 5 or 0, align = "center" }, children))
  barWidget.setTooltip(tooltip)
  barWidget.setVisible(true)
end

noctalia.state.watch(FRAME_KEY, render)
render(noctalia.state.get(FRAME_KEY))
