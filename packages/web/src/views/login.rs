use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use crate::Route;
use crate::application::AppAuthService;
use crate::domain::AuthCredentials;
use crate::application::AuthHooks;
use crate::i18n::{use_locale, LanguageSwitcher};

#[component]
pub fn LoginPage() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut login_result = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
    let mut show_forgot_modal = use_signal(|| false);

    let nav = use_navigator();

    let is_authenticated = AuthHooks::use_is_authenticated();
    use_effect(move || {
        if is_authenticated {
            nav.replace(Route::DashboardRoute {});
        }
    });

    let locale_ctx = use_locale();

    let t_welcome_back = locale_ctx.t("auth.welcome_back");
    let t_login_subtitle = locale_ctx.t("auth.login_subtitle");
    let t_email_label = locale_ctx.t("auth.email");
    let t_password_label = locale_ctx.t("auth.password");
    let t_forgot_password = locale_ctx.t("auth.forgot_password");
    let t_sign_in = locale_ctx.t("auth.sign_in");
    let t_signing_in = locale_ctx.t("auth.signing_in");
    let t_protected_by = locale_ctx.t("auth.protected_by");
    let t_validation_required = locale_ctx.t("validation.required");
    let t_invalid_credentials = locale_ctx.t("auth.invalid_credentials");
    let t_error_unknown = locale_ctx.t("errors.unknown");

    let result_class = if login_result.read().starts_with("✅") {
        "bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 border-green-200 dark:border-green-800"
    } else {
        "bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 border-red-200 dark:border-red-800"
    };

    rsx! {
        div {
            class: "min-h-screen w-full flex items-center justify-center bg-gradient-to-br from-indigo-50 via-purple-50 to-pink-50 dark:from-gray-900 dark:via-gray-900 dark:to-gray-800 p-3 sm:p-4 relative overflow-hidden",
            IridescenceBackground {}
            div {
                class: "hidden sm:block absolute top-20 left-20 w-72 h-72 bg-purple-300 dark:bg-purple-900/30 rounded-full mix-blend-multiply filter blur-xl opacity-70 animate-blob",
            }
            div {
                class: "hidden sm:block absolute top-20 right-20 w-72 h-72 bg-yellow-300 dark:bg-yellow-900/30 rounded-full mix-blend-multiply filter blur-xl opacity-70 animate-blob animation-delay-2000",
            }
            div {
                class: "hidden sm:block absolute -bottom-8 left-20 w-72 h-72 bg-pink-300 dark:bg-pink-900/30 rounded-full mix-blend-multiply filter blur-xl opacity-70 animate-blob animation-delay-4000",
            }

            div {
                class: "relative w-full max-w-md",
                div {
                    class: "bg-white/80 dark:bg-gray-800/80 backdrop-blur-xl rounded-2xl shadow-xl border border-white/20 dark:border-gray-700 p-5 sm:p-8 transform transition-all hover:scale-[1.01] duration-300",
                    div {
                        class: "text-center mb-8",
                        div {
                            class: "absolute top-4 right-4",
                            LanguageSwitcher {
                                class: "text-gray-600 dark:text-gray-300".to_string(),
                            }
                        }
                        div {
                            class: "w-16 h-16 sm:w-20 sm:h-20 mx-auto mb-4 bg-gradient-to-br from-primary to-purple-600 rounded-2xl shadow-lg shadow-primary/30 flex items-center justify-center transform rotate-3 hover:rotate-6 transition-all duration-300",
                            span {
                                class: "material-icons-outlined text-3xl sm:text-4xl text-white",
                                "school"
                            }
                        }
                        h1 {
                            class: "text-xl sm:text-2xl font-bold text-gray-900 dark:text-white mb-2",
                            "{t_welcome_back}"
                        }
                        p {
                            class: "text-gray-500 dark:text-gray-400",
                            "{t_login_subtitle}"
                        }
                    }

                    form {
                        class: "space-y-6",
                        action: "/api/auth/login",
                        method: "POST",
                        onsubmit: move |evt| {
                            evt.prevent_default();

                            let mut login_result = login_result;
                            let mut is_loading = is_loading;
                            let mut email_val = email.read().clone();
                            let mut password_val = password.read().clone();

                            if email_val.is_empty() || password_val.is_empty() {
                                if let Some(window) = web_sys::window() {
                                    if let Some(document) = window.document() {
                                        if email_val.is_empty() {
                                            if let Some(el) = document.get_element_by_id("login-email") {
                                                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                                    email_val = input.value();
                                                    email.set(email_val.clone());
                                                }
                                            }
                                        }
                                        if password_val.is_empty() {
                                            if let Some(el) = document.get_element_by_id("login-password") {
                                                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                                    password_val = input.value();
                                                    password.set(password_val.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if email_val.is_empty() || password_val.is_empty() {
                                login_result.set(t_validation_required.clone());
                                return;
                            }

                            is_loading.set(true);
                            login_result.set(String::new());

                            let t_invalid = t_invalid_credentials.clone();
                            let t_unknown = t_error_unknown.clone();

                            spawn(async move {
                                let credentials = AuthCredentials {
                                    email: email_val.clone(),
                                    password: password_val.clone(),
                                };

                                match AppAuthService::login(credentials).await {
                                    crate::domain::AuthResult::Success(_session) => {
                                        is_loading.set(false);
                                        nav.replace(Route::DashboardRoute {});
                                    }
                                    crate::domain::AuthResult::InvalidCredentials => {
                                        is_loading.set(false);
                                        login_result.set(t_invalid);
                                    }
                                    _ => {
                                        is_loading.set(false);
                                        login_result.set(t_unknown);
                                    }
                                }
                            });
                        },

                        div {
                            class: "space-y-2",
                            label {
                                class: "text-sm font-medium text-gray-700 dark:text-gray-300 block",
                                "{t_email_label}"
                            }
                            div {
                                class: "relative group",
                                span {
                                    class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-primary transition-colors",
                                    span { class: "material-icons-outlined text-lg", "email" }
                                }
                                input {
                                    id: "login-email",
                                    r#type: "email",
                                    class: "w-full pl-10 pr-4 py-3 bg-gray-50 dark:bg-gray-700/50 border border-gray-200 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-primary/50 focus:border-primary dark:focus:border-primary text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 transition-all outline-none",
                                    placeholder: "you@example.com",
                                    name: "email",
                                    autocomplete: "email",
                                    oninput: move |evt| email.set(evt.value()),
                                    disabled: is_loading
                                }
                            }
                        }

                        div {
                            class: "space-y-2",
                            div {
                                class: "flex justify-between items-center",
                                label {
                                    class: "text-sm font-medium text-gray-700 dark:text-gray-300",
                                    "{t_password_label}"
                                }
                                button {
                                    r#type: "button",
                                    class: "text-xs font-medium text-primary hover:text-primary-hover transition-colors",
                                    onclick: move |_| show_forgot_modal.set(true),
                                    "{t_forgot_password}"
                                }
                            }
                            div {
                                class: "relative group",
                                span {
                                    class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-primary transition-colors",
                                    span { class: "material-icons-outlined text-lg", "lock" }
                                }
                                input {
                                    id: "login-password",
                                    r#type: "password",
                                    class: "w-full pl-10 pr-4 py-3 bg-gray-50 dark:bg-gray-700/50 border border-gray-200 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-primary/50 focus:border-primary dark:focus:border-primary text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 transition-all outline-none",
                                    placeholder: "••••••••",
                                    name: "password",
                                    autocomplete: "current-password",
                                    oninput: move |evt| password.set(evt.value()),
                                    disabled: is_loading
                                }
                            }
                        }

                        if !login_result.read().is_empty() {
                            div {
                                class: "p-3 rounded-lg text-sm border {result_class} animate-fade-in flex items-center gap-2",
                                if login_result.read().starts_with("✅") {
                                    span { class: "material-icons-outlined", "check_circle" }
                                } else {
                                    span { class: "material-icons-outlined", "error" }
                                }
                                "{login_result}"
                            }
                        }

                        button {
                            r#type: "submit",
                            class: "w-full py-3.5 bg-gradient-to-r from-primary to-purple-600 hover:from-primary-hover hover:to-purple-700 text-white font-semibold rounded-xl shadow-lg shadow-primary/25 transform transition-all active:scale-[0.98] disabled:opacity-70 disabled:cursor-not-allowed flex items-center justify-center gap-2",
                            disabled: is_loading,
                            if is_loading() {
                                div {
                                    class: "w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin"
                                }
                                "{t_signing_in}"
                            } else {
                                "{t_sign_in}"
                                span { class: "material-icons-outlined", "arrow_forward" }
                            }
                        }
                    }

                    div {
                        class: "mt-8 pt-6 border-t border-gray-100 dark:border-gray-700 text-center",
                        p {
                            class: "text-xs text-gray-500 dark:text-gray-400",
                            "{t_protected_by}"
                        }
                    }
                }
            }
        }
        if show_forgot_modal() {
            ForgotPasswordModal {
                on_close: move |_| show_forgot_modal.set(false)
            }
        }
    }
}

/// Iridescence Background Component
#[component]
fn IridescenceBackground() -> Element {
    use_effect(move || {
        let script = r#"
            (async () => {
                try {
                    const container = document.getElementById('iridescence-container');
                    if (!container) return;
                    if (container._iridescence) return;

                    const { Renderer, Program, Mesh, Color, Triangle } = await import('https://esm.sh/ogl');

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

                    class IridescenceEffect {
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
                            this.gl.clearColor(0, 0, 0, 0);
                            this.container.appendChild(this.gl.canvas);
                            this.gl.canvas.style.display = 'block';
                            this.gl.canvas.style.width = '100%';
                            this.gl.canvas.style.height = '100%';
                            this.gl.canvas.style.position = 'absolute';
                            this.gl.canvas.style.top = '0';
                            this.gl.canvas.style.left = '0';
                            this.gl.canvas.style.zIndex = '0';

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
                            this.resize();
                            this.update = this.update.bind(this);
                            this.animateId = requestAnimationFrame(this.update);
                            this.handleMouseMove = this.handleMouseMove.bind(this);
                            if (this.options.mouseReact) {
                                window.addEventListener('mousemove', this.handleMouseMove);
                            }
                        }

                        resize() {
                            if (!this.container) return;
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
                            const x = e.clientX / window.innerWidth;
                            const y = 1.0 - (e.clientY / window.innerHeight);
                            this.mousePos = { x, y };
                            if (this.program) {
                                this.program.uniforms.uMouse.value[0] = x;
                                this.program.uniforms.uMouse.value[1] = y;
                            }
                        }
                    }

                    const effect = new IridescenceEffect(container, {
                        color: [1, 1, 1],
                        speed: 1.0,
                        amplitude: 0.1,
                        mouseReact: true
                    });
                    container._iridescence = effect;
                } catch (e) {
                    console.error("Failed to initialize Iridescence:", e);
                }
            })();
        "#;

        spawn(async move {
            let _ = document::eval(script);
        });
    });

    rsx! {
        div {
            id: "iridescence-container",
            class: "absolute inset-0 z-0 pointer-events-none",
            style: "width: 100%; height: 100%;",
        }
    }
}

/// Password reset is intentionally presented as unavailable until the email reset provider is wired.
#[component]
fn ForgotPasswordModal(on_close: EventHandler) -> Element {
    let locale = use_locale();
    let unavailable_message = if locale.is_rtl() {
        "بازنشانی رمز عبور از طریق ایمیل در این نسخه در دسترس نیست. برای دریافت راهنمایی با مدیر سیستم تماس بگیرید."
    } else {
        "Email password reset is unavailable in this release. Please contact your administrator for assistance."
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm animate-fade-in",
            onclick: move |_| on_close.call(()),
            div {
                class: "bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-gray-200 dark:border-gray-700 w-full max-w-md mx-4 p-6 animate-scale-in",
                onclick: move |e| e.stop_propagation(),
                div {
                    class: "flex items-center justify-between mb-6",
                    h2 { class: "text-xl font-bold text-gray-900 dark:text-white", "{locale.t(\"auth.reset_password\")}" }
                    button {
                        r#type: "button",
                        class: "p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                        onclick: move |_| on_close.call(()),
                        span { class: "material-icons-outlined", "close" }
                    }
                }
                div {
                    class: "space-y-6",
                    div {
                        class: "p-4 bg-gray-50 dark:bg-gray-800/70 rounded-lg border border-gray-200 dark:border-gray-700",
                        div {
                            class: "flex items-start gap-3",
                            span { class: "material-icons-outlined text-gray-500 dark:text-gray-400", "lock_reset" }
                            p { class: "text-sm text-gray-700 dark:text-gray-300", "{unavailable_message}" }
                        }
                    }
                    div {
                        class: "flex justify-end",
                        button {
                            r#type: "button",
                            class: "px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-xl hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors",
                            onclick: move |_| on_close.call(()),
                            "{locale.t(\"common.close\")}"
                        }
                    }
                }
            }
        }
    }
}
