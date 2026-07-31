--- dokidoki.base
--- =============
---
--- Small collection of general-purpose list and function utilities.

local base = {}

function base.range(first, last, step)
  step = step or 1
  local result = {}
  for i = first, last, step do
    result[#result+1] = i
  end
  return result
end

function base.ireverse(a)
  local result = {}
  local len = #a
  for i = 1, len do
    result[i] = a[len - i + 1]
  end
  return result
end

function base.map(f, t)
  local result = {}
  for k, v in pairs(t) do
    result[k] = f(v)
  end
  return result
end

function base.imap(f, a)
  local result = {}
  for i, v in ipairs(a) do
    result[i] = f(v)
  end
  return result
end

function base.ifoldl(f, init, a)
  for _, v in ipairs(a) do
    init = f(init, v)
  end
  return init
end

function base.iforeach(f, a)
  for i = 1, #a do
    f(a[i])
  end
end

function base.copy(t)
  local result = {}
  for k, v in pairs(t) do
    result[k] = v
  end
  return result
end

function base.irandomize(a)
  local result = base.copy(a)
  for i = 1, #result-1 do
    local j = math.random(i, #result)
    result[i], result[j] = result[j], result[i]
  end
  return result
end

function base.iconcat(a1, a2)
  local result = base.copy(a1)
  local len = #a1
  for i, v in ipairs(a2) do
    result[len + i] = v
  end
  return result
end

function base.ifilter(p, a)
  local result = {}
  for _, v in ipairs(a) do
    if p(v) then result[#result+1] = v end
  end
  return result
end

function base.build_array(len, f)
  local result = {}
  for i = 1, len do
    result[i] = f(i)
  end
  return result
end

function base.identity(...)
  return ...
end

function base.void()
end

function base.compose(f, ...)
  if select('#', ...) == 0 then
    return f
  else
    local rest = base.compose(...)
    return function (...)
      return f(rest(...))
    end
  end
end

return base
