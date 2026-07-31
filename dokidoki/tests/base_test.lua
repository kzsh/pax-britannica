local base = require "dokidoki.base"

local function all_equal(a, b)
  if a == b then
    return true
  elseif a == nil or b == nil then
    return false
  else
    for k, v in pairs(a) do
      if v ~= b[k] then
        return false
      end
    end
    for k, v in pairs(b) do
      if v ~= a[k] then
        return false
      end
    end
    return true
  end
end

local function square(x)
  return x * x
end

local function add1(x)
  return x + 1
end

-- first test my equality function :3
assert(all_equal({a = 1, b = 2}, {b = 2, a = 1}))
assert(not all_equal({a = 1, b = 2}, {b = 1, a = 2}))
assert(not all_equal({a = 1, b = 2}, {a = 1}))
assert(not all_equal({a = 1}, {a = 1, b = 2}))
assert(not all_equal({a = 1}, {}))
assert(not all_equal({}, {a = 1}))

assert(all_equal(base.range(1, 10), {1, 2, 3, 4, 5, 6, 7, 8, 9, 10}))
assert(all_equal(base.range(1, 4, 2), {1, 3}))
assert(all_equal(base.range(1, -5), {}))
assert(all_equal(base.range(1, -5, -2), {1, -1, -3, -5}))
assert(all_equal(base.ireverse{1, 2, 3}, {3, 2, 1}))
assert(all_equal(base.map(square, {4, 2, 3}), {16, 4, 9}))
assert(all_equal(base.map(add1, {x = 1, y = 4}), {x = 2, y = 5}))
assert(all_equal(base.imap(square, {4, 2, 3}), {16, 4, 9}))
assert(base.ifoldl(function (a, b) return a + b end, 0, {1, 2, 3}) == 6)
assert(all_equal(base.ifilter(base.identity, {1, true, false, "hello"}),
                 {1, true, "hello"}))
assert(all_equal(base.iconcat({1, 2}, {3, 4}), {1, 2, 3, 4}))
assert(all_equal(base.copy{a = 1, b = 2}, {a = 1, b = 2}))
assert(all_equal(base.irandomize{1}, {1}))
assert(#base.irandomize{1, 2, 3, 4} == 4)
assert(all_equal(base.build_array(3, square), {1, 4, 9}))
assert(all_equal(base.build_array(5, base.identity), {1, 2, 3, 4, 5}))
assert(all_equal({base.identity(1, "a", true)}, {1, "a", true}))
assert(base.void() == nil)
assert(base.compose(add1, square)(3) == 10)
assert(base.compose(add1)(3) == 4)

do
  local seen = {}
  base.iforeach(function (v) seen[#seen+1] = v end, {'a', 'b'})
  assert(all_equal(seen, {'a', 'b'}))
end

print("dokidoki.base: all tests passed")
