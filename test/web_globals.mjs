// The page runs two classic scripts in one global scope: miniquad's bundle and
// the inline script in index.html. A `const` in either is a global lexical
// binding, so a name used by both is a SyntaxError that kills the second script
// whole -- no fullscreen button, no load("pax.wasm"), and nothing in the logs
// but a 404 for the favicon. Nothing in the build catches that, so this does.
import { readFileSync } from "node:fs";
import { createContext, runInContext } from "node:vm";

const html = readFileSync("web/index.html", "utf8");
const bundle = readFileSync("web/dist/mq_js_bundle.js", "utf8");

const inline = [...html.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/g)].map(
  (m) => m[1],
);
if (inline.length !== 1) {
  throw new Error(`expected one inline script in web/index.html, found ${inline.length}`);
}

// A global that answers to anything, so the bundle's DOM and WebGL poking gets
// far enough to install its own top-level bindings. Only the binding names
// matter here; whatever the calls return does not.
const anything = new Proxy(function () {}, {
  get: (_t, key) => (key === Symbol.toPrimitive ? () => "" : anything),
  set: () => true,
  apply: () => anything,
  construct: () => anything,
  has: () => true,
});
const context = createContext(
  new Proxy(
    { console },
    {
      get: (target, key) => (key in target ? target[key] : anything),
      set: (target, key, value) => ((target[key] = value), true),
      // `const x` compiles against the real global, so claiming every name
      // exists would make every declaration a redeclaration. Only report the
      // names actually bound.
      has: (target, key) => key in target,
    },
  ),
);

runInContext(bundle, context, { filename: "mq_js_bundle.js" });
runInContext(inline[0], context, { filename: "index.html (inline)" });

console.log("web/index.html does not collide with mq_js_bundle.js globals");
