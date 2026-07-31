--- dokidoki.private.env
--- ====================
---
--- Manipulation of a function's global environment. Lua 5.1's `setfenv` is
--- gone; since 5.2 the environment of a Lua function is the upvalue named
--- `_ENV`, which this module gets and sets directly.

local env = {}

local function find_env_upvalue(f)
  local i = 1
  while true do
    local name = debug.getupvalue(f, i)
    if name == nil then return nil end
    if name == '_ENV' then return i end
    i = i + 1
  end
end

--- ### `get(f)`
--- Returns the environment of the Lua function `f`, or nil if it doesn't access
--- any globals.
function env.get(f)
  local i = find_env_upvalue(f)
  if i then
    local _, value = debug.getupvalue(f, i)
    return value
  end
end

--- ### `set(f, new_env)`
--- Points the environment of the Lua function `f` at `new_env`.
---
--- `f` gets a fresh `_ENV` upvalue rather than having the existing one
--- overwritten, so closures which used to share an environment with `f`
--- (including the chunk `f` was defined in) keep the environment they were
--- created with. This is what makes it possible to hand each script instance
--- its own environment and then put `f` back the way it was.
function env.set(f, new_env)
  local i = assert(find_env_upvalue(f),
                   "can't set the environment of a function without globals")
  debug.upvaluejoin(f, i, function () return new_env end, 1)
end

return env
