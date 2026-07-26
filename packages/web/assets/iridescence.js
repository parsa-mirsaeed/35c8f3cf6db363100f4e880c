
import { Renderer, Program, Mesh, Color, Triangle } from 'https://esm.sh/ogl';

const vertexShader = `
attribute vec2 uv;
attribute vec2 position;

varying vec2 vUv;

void main() {
  vUv = uv;
  gl_Position = vec4(position, 0, 1);
}
`;

const fragmentShader = `
precision highp float;

uniform float uTime;
uniform vec3 uColor;
uniform vec3 uResolution;
uniform vec2 uMouse;
uniform float uAmplitude;
uniform float uSpeed;

varying vec2 vUv;

void main() {
  float mr = min(uResolution.x, uResolution.y);
  vec2 uv = (vUv.xy * 2.0 - 1.0) * uResolution.xy / mr;

  uv += (uMouse - vec2(0.5)) * uAmplitude;

  float d = -uTime * 0.5 * uSpeed;
  float a = 0.0;
  for (float i = 0.0; i < 8.0; ++i) {
    a += cos(i - d - a * uv.x);
    d += sin(uv.y * i + a);
  }
  d += uTime * 0.5 * uSpeed;
  vec3 col = vec3(cos(uv * vec2(d, a)) * 0.6 + 0.4, cos(a + d) * 0.5 + 0.5);
  col = cos(col * cos(vec3(d, a, 2.5)) * 0.5 + 0.5) * uColor;
  gl_FragColor = vec4(col, 1.0);
}
`;

export class IridescenceEffect {
    constructor(container, options = {}) {
        this.container = container;
        this.options = {
            color: [1, 1, 1],
            speed: 1.0,
            amplitude: 0.1,
            mouseReact: true,
            ...options
        };
        this.mousePos = { x: 0.5, y: 0.5 };
        this.animateId = null;
        this.init();
    }

    init() {
        this.renderer = new Renderer({ alpha: true });
        this.gl = this.renderer.gl;
        this.gl.clearColor(0, 0, 0, 0); // Transparent background

        this.container.appendChild(this.gl.canvas);

        // Ensure canvas fills container absolutely
        this.gl.canvas.style.display = 'block';
        this.gl.canvas.style.width = '100%';
        this.gl.canvas.style.height = '100%';
        this.gl.canvas.style.position = 'absolute';
        this.gl.canvas.style.top = '0';
        this.gl.canvas.style.left = '0';
        this.gl.canvas.style.zIndex = '0'; // Behind everything initially

        this.resize = this.resize.bind(this);
        window.addEventListener('resize', this.resize, false);

        const geometry = new Triangle(this.gl);

        this.program = new Program(this.gl, {
            vertex: vertexShader,
            fragment: fragmentShader,
            uniforms: {
                uTime: { value: 0 },
                uColor: { value: new Color(...this.options.color) },
                uResolution: {
                    value: new Color(
                        this.gl.canvas.width,
                        this.gl.canvas.height,
                        this.gl.canvas.width / this.gl.canvas.height
                    )
                },
                uMouse: { value: new Float32Array([this.mousePos.x, this.mousePos.y]) },
                uAmplitude: { value: this.options.amplitude },
                uSpeed: { value: this.options.speed }
            }
        });

        this.mesh = new Mesh(this.gl, { geometry, program: this.program });

        // Initial resize
        this.resize();

        // Start loop
        this.update = this.update.bind(this);
        this.animateId = requestAnimationFrame(this.update);

        // Mouse handling
        this.handleMouseMove = this.handleMouseMove.bind(this);
        if (this.options.mouseReact) {
            window.addEventListener('mousemove', this.handleMouseMove);
        }
    }

    resize() {
        const width = this.container.offsetWidth;
        const height = this.container.offsetHeight;
        this.renderer.setSize(width, height);

        if (this.program) {
            this.program.uniforms.uResolution.value = new Color(
                this.gl.canvas.width,
                this.gl.canvas.height,
                this.gl.canvas.width / this.gl.canvas.height
            );
        }
    }

    update(t) {
        this.animateId = requestAnimationFrame(this.update);
        if (this.program) {
            this.program.uniforms.uTime.value = t * 0.001;
            this.renderer.render({ scene: this.mesh });
        }
    }

    handleMouseMove(e) {
        // Calculate standard normalized mouse/touch position
        // We use window coordinates since we're covering the whole screen usually
        // But let's map it primarily to the container if possible, or just window relative
        const x = e.clientX / window.innerWidth;
        const y = 1.0 - (e.clientY / window.innerHeight);

        this.mousePos = { x, y };

        if (this.program) {
            this.program.uniforms.uMouse.value[0] = x;
            this.program.uniforms.uMouse.value[1] = y;
        }
    }

    destroy() {
        cancelAnimationFrame(this.animateId);
        window.removeEventListener('resize', this.resize);
        if (this.options.mouseReact) {
            window.removeEventListener('mousemove', this.handleMouseMove);
        }
        if (this.container && this.gl && this.gl.canvas && this.container.contains(this.gl.canvas)) {
            this.container.removeChild(this.gl.canvas);
        }

        // Try to lose context to free memory
        const ext = this.gl.getExtension('WEBGL_lose_context');
        if (ext) ext.loseContext();
    }
}
