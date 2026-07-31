-- Headless stand-in for the native `collision.native` module.
--
-- A direct port of dokidoki-support/collision.c, so the separating axis results
-- match what the compiled game does.

local native = {}

local function dot(ax, ay, bx, by) return ax * bx + ay * by end

local function rotate_to(x, y, fx, fy)
  return x * fx + y * -fy, x * fy + y * fx
end

local function rotate_from(x, y, fx, fy)
  return x * fx + y * fy, x * -fy + y * fx
end

function native.make_polygon(coords)
  local poly = {bounding_radius = 0}
  for i = 1, #coords, 2 do
    local x, y = coords[i], coords[i+1]
    poly[#poly+1] = {x = x, y = y}
    local mag = math.sqrt(x * x + y * y)
    if poly.bounding_radius < mag then poly.bounding_radius = mag end
  end
  return poly
end

local function halfwidth_along_axis(ax, ay, poly)
  local hw = 0
  for _, v in ipairs(poly) do
    local new_hw = dot(v.x, v.y, ax, ay)
    if new_hw > hw then hw = new_hw end
  end
  return hw
end

local function separate_by_axis(ax, ay, hw1, body1, body2, out)
  local rx, ry = rotate_from(ax, ay, body2.fx, body2.fy)
  local overlap =
    dot(body1.x, body1.y, ax, ay) + hw1 +
    halfwidth_along_axis(-rx, -ry, body2.poly) -
    dot(body2.x, body2.y, ax, ay)
  if overlap <= 0 then return false end

  local scale = overlap / dot(ax, ay, ax, ay)
  local cx, cy = ax * scale, ay * scale
  if cx == 0 and cy == 0 then return false end

  if dot(cx, cy, cx, cy) < dot(out.x, out.y, out.x, out.y) then
    out.x, out.y = cx, cy
  end
  return true
end

local function separate_by_axes(body1, body2, out)
  local poly1 = body1.poly
  local j = #poly1
  for i = 1, #poly1 do
    local ax, ay = rotate_to(poly1[i].x - poly1[j].x, poly1[i].y - poly1[j].y,
                             0, -1)
    local hw1 = dot(poly1[i].x, poly1[i].y, ax, ay)
    ax, ay = rotate_to(ax, ay, body1.fx, body1.fy)
    if not separate_by_axis(ax, ay, hw1, body1, body2, out) then
      return false
    end
    j = i
  end
  return true
end

function native.collide(x1, y1, fx1, fy1, poly1, x2, y2, fx2, fy2, poly2)
  local body1 = {x = x1, y = y1, fx = fx1, fy = fy1, poly = poly1}
  local body2 = {x = x2, y = y2, fx = fx2, fy = fy2, poly = poly2}

  local bounding_distance = poly1.bounding_radius + poly2.bounding_radius
  local dx, dy = x2 - x1, y2 - y1
  local distance_squared = dx * dx + dy * dy
  if bounding_distance * bounding_distance < distance_squared then
    return false
  end

  local correction = {x = math.huge, y = math.huge}
  if not separate_by_axes(body1, body2, correction) or
     not separate_by_axes(body2, body1, correction) then
    return false
  end

  if dot(correction.x, correction.y, dx, dy) > 0 then
    correction.x, correction.y = -correction.x, -correction.y
  end

  return true, correction.x, correction.y
end

return native
