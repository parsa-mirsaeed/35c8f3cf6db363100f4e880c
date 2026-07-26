/** @type {import('tailwindcss').Config} */
module.exports = {
    mode: "all",
    content: [
        "./src/**/*.{rs,html,css}",
        "./dist/**/*.html",
        "./pages/**/*.rs",
        "./components/**/*.rs",
    ],
    darkMode: 'class', // or 'media' or 'class'
    theme: {
        extend: {
            fontFamily: {
                sans: ['Poppins', 'Inter', 'sans-serif'],
            },
            colors: {
                // Modern palette with slight saturation
                primary: {
                    light: '#a78bfa', // Violet 400
                    DEFAULT: '#8b5cf6', // Violet 500
                    dark: '#7c3aed', // Violet 600
                },
                secondary: {
                    light: '#f472b6', // Pink 400
                    DEFAULT: '#ec4899', // Pink 500
                    dark: '#db2777', // Pink 600
                },
                background: {
                    light: '#f3f4f6', // Gray 100
                    dark: '#111827', // Gray 900
                    paper_light: '#ffffff',
                    paper_dark: '#1f2937', // Gray 800
                },
            },
            keyframes: {
                'fade-in': {
                    '0%': { opacity: '0', transform: 'translateY(10px)' },
                    '100%': { opacity: '1', transform: 'translateY(0)' },
                },
                'scale-in': {
                    '0%': { transform: 'scale(0.95)', opacity: '0' },
                    '100%': { transform: 'scale(1)', opacity: '1' },
                },
            },
            animation: {
                'fade-in': 'fade-in 0.5s ease-out',
                'scale-in': 'scale-in 0.3s ease-out',
            },
            backdropBlur: {
                xs: '2px',
            },
        },
    },
    plugins: [
        require('@tailwindcss/forms'),
        require('@tailwindcss/typography'),
    ],
};
