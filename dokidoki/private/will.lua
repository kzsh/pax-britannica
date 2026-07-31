--- dokidoki.private.will
--- =====================
---
--- Attaches a finalizer to an arbitrary table. `fn` runs when the table it is
--- attached to becomes garbage.

local will = {}

local will_key =
  setmetatable({}, {__tostring = function () return "<private will>" end})

local function make_will(fn)
  -- a table with __gc in its metatable at setmetatable() time is marked for
  -- finalization
  return setmetatable({}, {
    __gc = function () fn() end,
    __tostring = function () return "<will>" end
  })
end

function will.attach_will(t, fn)
  t[will_key] = make_will(fn)
end

return will
