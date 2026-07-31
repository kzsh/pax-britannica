--- dokidoki.collision
--- ==================
---
--- Convex polygon collision detection.

local base = require 'dokidoki.base'
local v2 = require 'dokidoki.v2'

local native = require 'collision.native'

local collision = {}

--- ### `make_polygon(vertices)`
--- Creates a convex polygon from a list of `v2` points.
function collision.make_polygon(vertices)
  if v2.cross(vertices[2] - vertices[1], vertices[3] - vertices[2]) < 0 then
    vertices = base.ireverse(vertices)
  end

  local coords = {}
  for _, v in ipairs(vertices) do
    table.insert(coords, v.x)
    table.insert(coords, v.y)
  end
  return {data = native.make_polygon(coords), vertices = vertices}
end

--- ### `collide(body1, body2)`
--- Detects collision between a pair of bodies, where each body is of the form
--- `{pos=?, facing=?, poly=?}`. Returns false if there is no collision,
--- otherwise returns the vector which would pull `body1` out of `body2`.
function collision.collide(body1, body2)
  -- grr backwards compatibility
  local facing1 = body1.facing or v2.unit(body1.angle)
  local facing2 = body2.facing or v2.unit(body2.angle)

  local collided, x, y = native.collide(
    body1.pos.x, body1.pos.y, facing1.x, facing1.y, body1.poly.data,
    body2.pos.x, body2.pos.y, facing2.x, facing2.y, body2.poly.data)

  return collided and v2(x, y)
end

--- ### `points_to_polygon(points)`
--- Returns the centroid of `points` and a polygon relative to it.
function collision.points_to_polygon(points)
  local sum = v2(0, 0)
  for _, p in ipairs(points) do
    sum = sum + p
  end
  local pos = sum / #points
  local vertices = {}
  for _, p in ipairs(points) do
    table.insert(vertices, p - pos)
  end
  return pos, collision.make_polygon(vertices)
end

--- ### `make_rectangle(w, h)`
--- Creates a rectangular polygon centered on the origin.
function collision.make_rectangle(w, h)
  return collision.make_polygon{
    v2(-w/2, -h/2),
    v2(w/2, -h/2),
    v2(w/2, h/2),
    v2(-w/2, h/2)}
end

return collision
