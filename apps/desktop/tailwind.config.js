/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: "#0B0F14",
        surface: "#111823",
        surface2: "#17202C",
        line: "#243244",
        ink: "#E6EDF5",
        muted: "#93A4B8",
        blue: {
          DEFAULT: "#3B82F6",
          100: "#DBEAFE",
          200: "#BFDBFE",
          300: "#93C5FD",
        },
        blueSoft: "#1E3A5F",
        red: {
          DEFAULT: "#EF4444",
          100: "#FEE2E2",
          200: "#FECACA",
          300: "#FCA5A5",
        },
        redSoft: "#4A1D1D",
        gold: "#D4AF37",
        success: "#22C55E",
        warning: "#F59E0B",
        danger: "#EF4444",
      },
      fontFamily: {
        sans: [
          "Inter",
          "Segoe UI",
          "system-ui",
          "-apple-system",
          "sans-serif",
        ],
      },
      boxShadow: {
        panel: "0 18px 50px -24px rgba(0, 0, 0, 0.85)",
        glow: "0 0 0 1px rgba(212, 175, 55, 0.35), 0 12px 36px -18px rgba(212, 175, 55, 0.35)",
      },
      keyframes: {
        rise: {
          "0%": { opacity: "0", transform: "translateY(6px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
      animation: {
        rise: "rise 180ms ease-out",
      },
    },
  },
  plugins: [],
};
