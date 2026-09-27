/**
 * Aurora shader — domain-warped fractal noise folded into soft ribbons.
 *
 * Three accent colours (uniforms) are blended along the warped field; the
 * cursor adds a gentle light bloom. Cheap enough for integrated GPUs at half
 * resolution: 4 octaves of value noise, no loops over lights, no textures.
 */

export const AURORA_VERTEX = /* glsl */ `#version 300 es
in vec2 aPosition;
out vec2 vUv;
void main() {
  vUv = aPosition * 0.5 + 0.5;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

export const AURORA_FRAGMENT = /* glsl */ `#version 300 es
precision highp float;

in vec2 vUv;
out vec4 outColor;

uniform float uTime;
uniform vec2 uResolution;
uniform vec2 uMouse;        // 0..1, y up
uniform float uMouseEnergy; // 0..1, decays when the pointer rests
uniform vec3 uColorA;
uniform vec3 uColorB;
uniform vec3 uColorC;
uniform vec3 uBase;
uniform float uIntensity;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
  float v = 0.0;
  float amp = 0.5;
  mat2 rot = mat2(0.8, -0.6, 0.6, 0.8);
  for (int i = 0; i < 4; i++) {
    v += amp * noise(p);
    p = rot * p * 2.02 + 3.1;
    amp *= 0.5;
  }
  return v;
}

void main() {
  vec2 uv = vUv;
  float aspect = uResolution.x / max(uResolution.y, 1.0);
  vec2 p = vec2(uv.x * aspect, uv.y) * 1.6;
  float t = uTime * 0.045;

  // Domain warping: two nested fbm lookups give the flowing curtain shapes.
  vec2 q = vec2(fbm(p + vec2(0.0, t)), fbm(p + vec2(5.2, -t * 1.3)));
  vec2 r = vec2(fbm(p + 3.5 * q + vec2(1.7, 9.2) + t * 0.6), fbm(p + 3.5 * q + vec2(8.3, 2.8) - t * 0.4));
  float f = fbm(p + 2.8 * r);

  // Ribbons: sharpen bands of the field into luminous curtains.
  float ribbon = smoothstep(0.35, 0.95, f) * (0.55 + 0.45 * sin(6.2831 * (r.x + t * 0.5)));
  ribbon = clamp(ribbon, 0.0, 1.0);

  vec3 col = mix(uColorA, uColorB, smoothstep(0.1, 0.9, q.x));
  col = mix(col, uColorC, smoothstep(0.55, 1.0, r.y) * 0.75);

  // Vertical falloff: aurora lives in the upper two thirds, the bottom stays deep.
  float vertical = smoothstep(-0.15, 0.85, uv.y);
  vec3 aurora = col * ribbon * vertical * uIntensity;

  // Cursor bloom.
  vec2 m = vec2(uMouse.x * aspect, uMouse.y);
  float d = distance(vec2(uv.x * aspect, uv.y), m);
  float bloom = exp(-d * d * 9.0) * 0.22 * uMouseEnergy;
  aurora += mix(uColorA, uColorC, 0.5) * bloom;

  // Vignette + base.
  float vig = smoothstep(1.35, 0.25, length((uv - 0.5) * vec2(aspect * 0.9, 1.1)));
  vec3 color = uBase + aurora * vig;

  // Tone map + tiny dither against banding.
  color = color / (1.0 + color * 0.6);
  color += (hash(gl_FragCoord.xy + uTime) - 0.5) / 255.0;
  outColor = vec4(color, 1.0);
}
`;
