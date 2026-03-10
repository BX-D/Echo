/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        void: "#0a0a0a",
        shadow: "#1a1a1a",
        ash: "#2a2a2a",
        smoke: "#666666",
        bone: "#d4d0c8",
        parchment: "#e8e0d4",
        blood: "#8b0000",
        rust: "#a0522d",
        bile: "#556b2f",
        clinical: "#f0f8ff",
        bruise: "#4a0e4e",
        gangrene: "#2f4f4f",
      },
      fontFamily: {
        horror: ['"Special Elite"', '"Creepster"', "monospace"],
        body: ['"IBM Plex Mono"', "monospace"],
      },
      animation: {
        flicker: "flicker 0.15s infinite",
        glitch: "glitch 2.5s infinite",
        pulse_slow: "pulse 4s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
      keyframes: {
        flicker: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.8" },
        },
        glitch: {
          "0%, 100%": { transform: "translate(0)" },
          "20%": { transform: "translate(-2px, 2px)" },
          "40%": { transform: "translate(-2px, -2px)" },
          "60%": { transform: "translate(2px, 2px)" },
          "80%": { transform: "translate(2px, -2px)" },
        },
      },
    },
  },
  plugins: [],
};
