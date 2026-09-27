import { useEffect, useRef } from "react";
import { selectFxRunning, useFx } from "@/stores/fx";
import { AURORA_FRAGMENT, AURORA_VERTEX } from "./shaders";

/**
 * Full-window WebGL2 aurora behind the whole app.
 *
 * Budget rules (the app runs next to a game):
 * - renders at a fraction of the device resolution (0.5 high / 0.33 low),
 * - caps the frame rate (60 high / 30 low),
 * - stops the RAF loop entirely when paused (game focused, window hidden,
 *   quality "off") and keeps the last frame on screen,
 * - `prefers-reduced-motion` renders one still frame.
 * If WebGL2 is unavailable the CSS gradient fallback below stays visible.
 */

const BASE_COLOR: [number, number, number] = [0.018, 0.02, 0.04];

function hexToRgb(hex: string): [number, number, number] {
  const n = Number.parseInt(hex.replace("#", ""), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.warn("[aurora] shader compile failed", gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

export function AuroraCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const running = useFx(selectFxRunning);
  const quality = useFx((s) => s.quality);
  const accents = useFx((s) => s.accents);

  // Live values read by the render loop without re-creating the GL context.
  const live = useRef({ running, quality, accents, mouse: [0.5, 0.6], energy: 0, targetEnergy: 0 });
  live.current.running = running;
  live.current.quality = quality;
  live.current.accents = accents;

  const kick = useRef<() => void>(() => {});

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const gl = canvas.getContext("webgl2", { antialias: false, alpha: false, powerPreference: "low-power" });
    if (!gl) return;

    const vs = compile(gl, gl.VERTEX_SHADER, AURORA_VERTEX);
    const fs = compile(gl, gl.FRAGMENT_SHADER, AURORA_FRAGMENT);
    if (!vs || !fs) return;
    const program = gl.createProgram();
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.warn("[aurora] link failed", gl.getProgramInfoLog(program));
      return;
    }
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL call, not a React hook
    gl.useProgram(program);

    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(program, "aPosition");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

    const u = (name: string) => gl.getUniformLocation(program, name);
    const uTime = u("uTime");
    const uResolution = u("uResolution");
    const uMouse = u("uMouse");
    const uMouseEnergy = u("uMouseEnergy");
    const uColorA = u("uColorA");
    const uColorB = u("uColorB");
    const uColorC = u("uColorC");
    const uBase = u("uBase");
    const uIntensity = u("uIntensity");

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const start = performance.now();
    let raf = 0;
    let last = 0;
    // Smoothed colours so palette changes fade instead of snapping.
    const colors = live.current.accents.map(hexToRgb);

    const resize = () => {
      const scale = live.current.quality === "low" ? 0.33 : 0.5;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.max(1, Math.floor(window.innerWidth * dpr * scale));
      const h = Math.max(1, Math.floor(window.innerHeight * dpr * scale));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl.viewport(0, 0, w, h);
      }
    };

    const draw = (now: number) => {
      const s = live.current;
      const targets = s.accents.map(hexToRgb);
      for (let i = 0; i < 3; i++) {
        const c = colors[i];
        const t = targets[i];
        if (!c || !t) continue;
        for (let k = 0; k < 3; k++) c[k] = (c[k] ?? 0) + ((t[k] ?? 0) - (c[k] ?? 0)) * 0.04;
      }
      s.energy += (s.targetEnergy - s.energy) * 0.05;
      s.targetEnergy *= 0.985;

      gl.uniform1f(uTime, (now - start) / 1000);
      gl.uniform2f(uResolution, canvas.width, canvas.height);
      gl.uniform2f(uMouse, s.mouse[0] ?? 0.5, s.mouse[1] ?? 0.5);
      gl.uniform1f(uMouseEnergy, s.energy);
      gl.uniform3fv(uColorA, colors[0] ?? [0.5, 0.4, 1]);
      gl.uniform3fv(uColorB, colors[1] ?? [0.2, 0.5, 1]);
      gl.uniform3fv(uColorC, colors[2] ?? [1, 0.5, 0.1]);
      gl.uniform3fv(uBase, BASE_COLOR);
      gl.uniform1f(uIntensity, 0.9);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };

    const loop = (now: number) => {
      raf = 0;
      const s = live.current;
      if (!s.running || s.quality === "off") return; // paused: keep last frame
      const minFrame = s.quality === "low" ? 1000 / 30 : 1000 / 60;
      if (now - last >= minFrame - 1) {
        last = now;
        resize();
        draw(now);
      }
      if (!reduced.matches) raf = requestAnimationFrame(loop);
    };

    kick.current = () => {
      if (!raf) raf = requestAnimationFrame(loop);
    };

    const onPointer = (e: PointerEvent) => {
      const s = live.current;
      s.mouse = [e.clientX / window.innerWidth, 1 - e.clientY / window.innerHeight];
      s.targetEnergy = 1;
    };
    const onResize = () => {
      resize();
      kick.current();
    };

    window.addEventListener("pointermove", onPointer, { passive: true });
    window.addEventListener("resize", onResize);
    resize();
    draw(performance.now()); // always paint one frame, even when paused
    kick.current();

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("resize", onResize);
      gl.deleteProgram(program);
      gl.deleteShader(vs);
      gl.deleteShader(fs);
      gl.deleteBuffer(buffer);
    };
  }, []);

  // Resume the loop when un-paused or when quality changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: quality is a deliberate re-trigger
  useEffect(() => {
    if (running) kick.current();
  }, [running, quality]);

  return (
    <div aria-hidden className="pointer-events-none fixed inset-0 -z-10">
      {/* CSS fallback (no WebGL2 / before first frame). */}
      <div className="absolute inset-0 bg-[radial-gradient(1200px_600px_at_20%_-10%,color-mix(in_oklab,var(--color-accent)_35%,transparent),transparent),radial-gradient(900px_500px_at_90%_10%,color-mix(in_oklab,var(--color-accent-3)_22%,transparent),transparent),var(--color-bg)]" />
      <canvas
        ref={canvasRef}
        className="absolute inset-0 h-full w-full transition-opacity duration-700"
        style={{ opacity: quality === "off" ? 0 : 1 }}
      />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_bottom,transparent_40%,rgb(0_0_0/0.55))]" />
    </div>
  );
}
