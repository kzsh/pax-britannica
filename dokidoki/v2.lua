--- dokidoki.v2
--- ===========
---
--- Immutable 2d vectors. The module itself is callable: `v2(x, y)` is the same
--- as `v2.make(x, y)`.

local v2 = {}

local mt

function v2.make(x, y) return setmetatable({x=x, y=y}, mt) end

local make = v2.make

function v2.unit(angle) return make(math.cos(angle), math.sin(angle)) end

function v2.add(a, b)   return make(a.x + b.x, a.y + b.y) end
function v2.sub(a, b)   return make(a.x - b.x, a.y - b.y) end
function v2.neg(v)      return make(-v.x, -v.y) end
function v2.mul(v, s)   return make(s * v.x, s * v.y) end
function v2.div(v, s)   return make(v.x / s, v.y / s) end
function v2.dot(a, b)   return a.x * b.x + a.y * b.y end
function v2.cross(a, b) return a.x * b.y - a.y * b.x end

function v2.sqrmag(v)   return v2.dot(v, v) end
function v2.mag(v)      return math.sqrt(v.x * v.x + v.y * v.y) end
function v2.angle(v)    return math.atan(v.y, v.x) end
function v2.norm(v)     return v2.div(v, v2.mag(v)) end
function v2.eq(a, b)    return a.x == b.x and a.y == b.y end
function v2.coords(v)   return v.x, v.y end

function v2.project(a, b) return v2.mul(b, v2.dot(a, b) / v2.sqrmag(b)) end

function v2.rotate(v, a)
  local sin_a, cos_a = math.sin(a), math.cos(a)
  return make(v.x * cos_a - v.y * sin_a, v.y * cos_a + v.x * sin_a)
end

function v2.rotate90(v)
  return make(-v.y, v.x)
end

function v2.rotate_to(v, i)
  return make(i.x * v.x - i.y * v.y, i.y * v.x + i.x * v.y)
end

function v2.rotate_from(v, i)
  return make(i.x * v.x + i.y * v.y, - i.y * v.x + i.x * v.y)
end

function v2.random()
  return v2.mul(v2.unit(math.random() * math.pi * 2), math.sqrt(math.random()))
end

mt =
{
  __add = v2.add,
  __sub = v2.sub,
  __mul = function (a, b)
    return type(a) == 'number' and v2.mul(b, a) or v2.mul(a, b)
  end,
  __div = v2.div,
  __unm = v2.neg,
  __eq = v2.eq,

  __tostring = function (v)
    return '(' .. v.x .. ', ' .. v.y .. ')'
  end,
}

v2.zero = make(0, 0)
v2.i = make(1, 0)
v2.j = make(0, 1)

return setmetatable(v2, {__call = function (_, x, y) return make(x, y) end})
