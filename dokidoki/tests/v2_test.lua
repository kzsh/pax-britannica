local v2 = require "dokidoki.v2"

assert(v2.zero.x == 0);
assert(v2.zero.y == 0);
assert(v2.i.x == 1);
assert(v2.i.y == 0);
assert(v2.j.x == 0);
assert(v2.j.y == 1);

assert(v2(1, 2) == v2(1, 2))
assert(v2(1, 2) ~= v2(10, 20))
assert(v2(1, 2) + v2(10, 20) == v2(11, 22))
assert(v2(1, 2) - v2(10, 20) == v2(-9, -18))
assert(-v2(1, 2) == v2(-1, -2))
assert(v2.i - v2.j == -v2(-1, 1))
assert(v2(1, 2) * 2 == v2(2, 4))
assert(v2(1, 2) / 2 == v2(0.5, 1))
assert(v2.dot(v2.i, v2.i) == 1)
assert(v2.dot(v2.i, -v2.i) == -1)
assert(v2.dot(v2.j, v2.j) == 1)
assert(v2.dot(v2.i, v2.j) == 0)
assert(v2.dot(v2(1, 2), v2(10, 20)) == 50)
assert(v2.dot(v2.i, v2.zero) == 0)
assert(v2.cross(v2.i, v2.i) == 0)
assert(v2.cross(v2.i, v2.zero) == 0)
assert(v2.cross(v2.j, v2.j) == 0)
assert(v2.cross(v2.i, v2.j) == 1)
assert(v2.cross(v2.i, v2.j * 2) == 2)
assert(v2.mag(v2.i) == 1)
assert(v2.mag(v2.zero) == 0)
assert(v2.mag(v2(3, 4)) == 5)
assert(v2.sqrmag(v2(3, 4)) == 25)
assert(v2.mag(v2.unit(0.5) ) - 1 < 1e-12)
assert(math.abs(v2.angle(v2(0, 1)) - math.pi/2) < 1e-12)
assert(v2.norm(v2(3, 4)) == v2(0.6, 0.8))
assert(v2.rotate90(v2.i) == v2.j)
assert(v2.rotate_to(v2.i, v2.j) == v2.j)
assert(v2.rotate_from(v2.j, v2.j) == v2.i)
assert(v2.project(v2(2, 3), v2.i) == v2(2, 0))
assert(v2.mag(v2.random()) <= 1)
assert(tostring(v2(1, 2)) == "(1, 2)")
assert(select("#", v2.coords(v2(1, 2))) == 2)
assert(v2.make(1, 2) == v2(1, 2))
do
  local x, y = v2.coords(v2(5, 6))
  assert(x == 5 and y == 6)
end
print("dokidoki.v2: all tests passed")
