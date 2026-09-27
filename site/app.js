// Site behaviour: the WebGL "pitch" backdrops, the hybrid decal showcase,
// scroll reveals and the latest-release download link. Everything degrades
// to a static page without JavaScript or WebGL.

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

// ── Pitch shader ───────────────────────────────────────────────────────
// Sky: domain-warped aurora (same field as the app's background). Ground:
// a perspective pitch whose lines scroll toward the viewer, blue half on
// the left, orange half on the right, lit by the cursor.
const VERTEX = `#version 300 es
in vec2 p;
void main() { gl_Position = vec4(p, 0.0, 1.0); }`;

const FRAGMENT = `#version 300 es
precision highp float;
out vec4 o;
uniform vec2 uRes;
uniform float uTime;
uniform vec2 uMouse;
uniform float uEnergy;
uniform float uHorizon;
uniform float uSky;

const vec3 BLUE = vec3(0.18, 0.48, 1.0);
const vec3 ORANGE = vec3(1.0, 0.45, 0.1);
const vec3 VIOLET = vec3(0.49, 0.42, 1.0);

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  mat2 r = mat2(0.8, -0.6, 0.6, 0.8);
  for (int i = 0; i < 4; i++) {
    v += a * noise(p);
    p = r * p * 2.02 + 3.1;
    a *= 0.5;
  }
  return v;
}

void main() {
  vec2 uv = gl_FragCoord.xy / uRes;
  float aspect = uRes.x / uRes.y;
  float t = uTime * 0.04;
  vec3 col = vec3(0.016, 0.018, 0.03);

  // Aurora.
  vec2 p = vec2(uv.x * aspect, uv.y) * 1.5;
  vec2 q = vec2(fbm(p + vec2(0.0, t)), fbm(p + vec2(5.2, -t * 1.3)));
  vec2 r = vec2(fbm(p + 3.5 * q + vec2(1.7, 9.2) + t * 0.6), fbm(p + 3.5 * q + vec2(8.3, 2.8) - t * 0.4));
  float f = fbm(p + 2.8 * r);
  float ribbon = clamp(smoothstep(0.35, 0.95, f) * (0.55 + 0.45 * sin(6.2831 * (r.x + t * 0.5))), 0.0, 1.0);
  vec3 aurora = mix(BLUE, VIOLET, smoothstep(0.1, 0.9, q.x));
  aurora = mix(aurora, ORANGE, smoothstep(0.55, 1.0, r.y) * 0.7);
  float sky = smoothstep(uHorizon - 0.02, uHorizon + 0.5, uv.y);
  col += aurora * ribbon * sky * uSky;

  // Horizon line, team coloured.
  float dh = uv.y - uHorizon;
  vec3 team = mix(BLUE, ORANGE, smoothstep(0.3, 0.7, uv.x));
  col += team * exp(-abs(dh) * 60.0) * 0.45 + team * exp(-abs(dh) * 9.0) * 0.06;

  // Ground: perspective grid.
  if (dh < 0.0) {
    float depth = 0.32 / (uHorizon - uv.y);
    vec2 w = vec2((uv.x - 0.5) * aspect * depth, depth + uTime * 0.35);
    vec2 g = w * 1.4;
    vec2 d = abs(fract(g - 0.5) - 0.5) / max(fwidth(g), vec2(1e-4));
    float line = 1.0 - min(min(d.x, d.y), 1.0);
    float fog = exp(-depth * 0.28);
    vec3 side = mix(BLUE, ORANGE, smoothstep(-0.8, 0.8, w.x));
    float mid = 1.0 - min(abs(w.x) / max(fwidth(w.x) * 1.4, 1e-4), 1.0);

    // Cursor lights the pitch.
    vec2 m = vec2(uMouse.x * aspect, uMouse.y);
    float near = exp(-pow(distance(vec2(uv.x * aspect, uv.y), m), 2.0) * 14.0) * uEnergy;

    col += side * line * fog * (0.75 + near * 1.6);
    col += vec3(0.85) * mid * fog * 0.3;
    col += aurora * ribbon * 0.07 * fog;
  }

  // Cursor bloom in the sky.
  vec2 m2 = vec2(uMouse.x * aspect, uMouse.y);
  float dm = distance(vec2(uv.x * aspect, uv.y), m2);
  col += mix(BLUE, ORANGE, uMouse.x) * exp(-dm * dm * 10.0) * 0.14 * uEnergy;

  float vig = smoothstep(1.4, 0.3, length((uv - vec2(0.5, 0.45)) * vec2(aspect * 0.8, 1.1)));
  col *= vig;
  col = col / (1.0 + col * 0.6);
  col += (hash(gl_FragCoord.xy + uTime) - 0.5) / 255.0;
  o = vec4(col, 1.0);
}`;

const FIELDS = {
  hero: { horizon: 0.2, sky: 1.15 },
  finale: { horizon: 0.42, sky: 0.6 },
};

function startField(canvas) {
  const cfg = FIELDS[canvas.dataset.field] || FIELDS.hero;
  const gl = canvas.getContext("webgl2", { antialias: false, alpha: false, powerPreference: "low-power" });
  if (!gl) return;
  const shader = (type, src) => {
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    return gl.getShaderParameter(s, gl.COMPILE_STATUS) ? s : null;
  };
  const vs = shader(gl.VERTEX_SHADER, VERTEX);
  const fs = shader(gl.FRAGMENT_SHADER, FRAGMENT);
  if (!vs || !fs) return;
  const prog = gl.createProgram();
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.bindAttribLocation(prog, 0, "p");
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return;
  gl.useProgram(prog);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  const u = (n) => gl.getUniformLocation(prog, n);
  const uRes = u("uRes");
  const uTime = u("uTime");
  const uMouse = u("uMouse");
  const uEnergy = u("uEnergy");
  gl.uniform1f(u("uHorizon"), cfg.horizon);
  gl.uniform1f(u("uSky"), cfg.sky);

  const state = { visible: false, raf: 0, mouse: [0.5, 0.3], energy: 0, target: 0 };
  const start = performance.now();

  const resize = () => {
    const scale = Math.min(window.devicePixelRatio || 1, 1.5) * 0.7;
    const w = Math.max(1, Math.round(canvas.clientWidth * scale));
    const h = Math.max(1, Math.round(canvas.clientHeight * scale));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      gl.viewport(0, 0, w, h);
    }
  };

  const draw = (now) => {
    state.energy += (state.target - state.energy) * 0.06;
    state.target *= 0.985;
    resize();
    gl.uniform2f(uRes, canvas.width, canvas.height);
    gl.uniform1f(uTime, (now - start) / 1000);
    gl.uniform2f(uMouse, state.mouse[0], state.mouse[1]);
    gl.uniform1f(uEnergy, state.energy);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  };

  const loop = (now) => {
    state.raf = 0;
    if (!state.visible) return;
    draw(now);
    state.raf = requestAnimationFrame(loop);
  };

  draw(start + 12000);
  canvas.classList.add("ready");
  if (reducedMotion) return;

  new IntersectionObserver(([entry]) => {
    state.visible = entry.isIntersecting && !document.hidden;
    if (state.visible && !state.raf) state.raf = requestAnimationFrame(loop);
  }).observe(canvas);

  canvas.parentElement.parentElement.addEventListener(
    "pointermove",
    (e) => {
      const r = canvas.getBoundingClientRect();
      state.mouse = [(e.clientX - r.left) / r.width, 1 - (e.clientY - r.top) / r.height];
      state.target = 1;
    },
    { passive: true },
  );
}

for (const canvas of document.querySelectorAll("canvas[data-field]")) startField(canvas);

// ── Showcase: paint switcher + tilt ────────────────────────────────────
const showcase = document.querySelector(".showcase");
if (showcase) {
  const shots = showcase.querySelectorAll(".shot");
  const buttons = showcase.querySelectorAll(".paint");
  let current = 0;
  let auto = !reducedMotion;
  const select = (i) => {
    current = i;
    shots.forEach((s, k) => s.classList.toggle("on", k === i));
    buttons.forEach((b, k) => b.setAttribute("aria-pressed", String(k === i)));
  };
  buttons.forEach((b, i) =>
    b.addEventListener("click", () => {
      auto = false;
      select(i);
    }),
  );
  setInterval(() => {
    if (auto && !document.hidden) select((current + 1) % shots.length);
  }, 3800);

  const frame = showcase.querySelector(".frame");
  if (!reducedMotion && window.matchMedia("(pointer: fine)").matches) {
    showcase.addEventListener("pointermove", (e) => {
      const r = frame.getBoundingClientRect();
      const x = (e.clientX - r.left) / r.width;
      const y = (e.clientY - r.top) / r.height;
      frame.style.setProperty("--ry", `${(x - 0.5) * 7}deg`);
      frame.style.setProperty("--rx", `${(0.5 - y) * 6}deg`);
      frame.style.setProperty("--gx", `${x * 100}%`);
      frame.style.setProperty("--gy", `${y * 100}%`);
    });
    showcase.addEventListener("pointerleave", () => {
      frame.style.setProperty("--ry", "0deg");
      frame.style.setProperty("--rx", "0deg");
    });
  }
}

// ── Tile spotlight ─────────────────────────────────────────────────────
for (const tile of document.querySelectorAll(".tile")) {
  tile.addEventListener("pointermove", (e) => {
    const r = tile.getBoundingClientRect();
    tile.style.setProperty("--mx", `${e.clientX - r.left}px`);
    tile.style.setProperty("--my", `${e.clientY - r.top}px`);
  });
}

// ── Reveal on scroll ───────────────────────────────────────────────────
const reveal = new IntersectionObserver(
  (entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      e.target.classList.add("in");
      reveal.unobserve(e.target);
    }
  },
  { rootMargin: "0px 0px -8% 0px" },
);
document.querySelectorAll("[data-reveal]").forEach((el) => {
  const siblings = [...el.parentElement.children].filter((c) => c.hasAttribute("data-reveal"));
  el.style.setProperty("--delay", `${Math.min(siblings.indexOf(el), 5) * 70}ms`);
  reveal.observe(el);
});

// ── Header ─────────────────────────────────────────────────────────────
const header = document.querySelector("header.top");
const onScroll = () => header.classList.toggle("scrolled", window.scrollY > 24);
window.addEventListener("scroll", onScroll, { passive: true });
onScroll();

for (const el of document.querySelectorAll("[data-year]")) {
  el.textContent = String(new Date().getFullYear());
}

// ── Latest release ─────────────────────────────────────────────────────
// Points the download buttons at the installer of the latest release and
// shows its version. Without JavaScript (or if the GitHub API is rate
// limited) the buttons keep linking to the latest release page.
(async () => {
  try {
    const res = await fetch("https://api.github.com/repos/ALXS-GitHub/ALXS-RL-Mod/releases/latest", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return;
    const release = await res.json();
    const installer = (release.assets || []).find((a) => /setup\.exe$/i.test(a.name));
    if (installer) {
      for (const b of document.querySelectorAll("[data-download]")) b.href = installer.browser_download_url;
    }
    for (const v of document.querySelectorAll("[data-version]")) v.textContent = release.tag_name;
  } catch {
    // Keep the static links.
  }
})();
