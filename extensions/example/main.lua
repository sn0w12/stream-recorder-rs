---@type HookModule
local M = {}

---@param ctx HookContext
function M.after_recording(ctx)
  local recording = ctx.recording
  local seconds = recording and recording.duration_seconds or 0
  local minutes = math.floor(seconds / 60)

  sr.template.set("duration_minutes", minutes)
  sr.log.info(string.format("%s: %d minutes", ctx.settings.label, minutes))
end

return M
