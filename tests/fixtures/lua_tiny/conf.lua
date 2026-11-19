-- Fixture for the Lua/LÖVE backend E2E check. Every generated argument, including
-- non-table values, must produce a deterministic result so the recording is reproducible.
function love.conf(t)
  if type(t) == "table" then
    t.window = t.window or {}
    t.window.width = 1280
  end
end
