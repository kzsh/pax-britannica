-- Prints test vectors for src/rng.rs, straight out of the Lua 5.4 the port has
-- to match. Regenerate with: lua5.4 test/rng_vectors.lua
--
-- The Rust tests hardcode this output. That is deliberate: the point is to
-- compare against the real interpreter's behaviour, so the expected values must
-- come from the interpreter and not from a second implementation of the
-- algorithm.

local function draws(seed, n, f)
  math.randomseed(seed)
  local out = {}
  for _ = 1, n do out[#out+1] = f() end
  return table.concat(out, ', ')
end

local function float() return string.format('%.17g', math.random()) end

print('seed 1, math.random():')
print('  ' .. draws(1, 8, float))
print('seed 42, math.random():')
print('  ' .. draws(42, 8, float))
print('seed 1, math.random(1, 100):')
print('  ' .. draws(1, 8, function () return math.random(1, 100) end))
print('seed 1, math.random(0, 7)  -- power-of-two fast path:')
print('  ' .. draws(1, 8, function () return math.random(0, 7) end))
print('seed 1, math.random(mininteger, maxinteger)  -- width overflows i64:')
print('  ' .. draws(1, 4, function ()
  return math.random(math.mininteger, math.maxinteger)
end))
