/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Paleta tomada de la interfaz de League of Legends: azul noche,
        // crema para el texto, latón dorado para los ornamentos y los
        // colores de bando turquesa/rojo del champ select.
        bg: "#010A13",
        surface: "#0A1428",
        surface2: "#0D1B2E",
        line: "#1E283D",
        ink: "#F0E6D2",
        muted: "#A09B8C",
        blue: {
          DEFAULT: "#0AC8B9",
          100: "#C4F7F2",
          200: "#8CEDE4",
          300: "#4FDDD1",
        },
        blueSoft: "#06322F",
        red: {
          DEFAULT: "#E84057",
          100: "#FCE1E5",
          200: "#F9BCC4",
          300: "#F4909D",
        },
        redSoft: "#3A121B",
        gold: "#C89B3C",
        success: "#3DDC97",
        warning: "#E5A83C",
        danger: "#E84057",
      },
      // El cliente de League no redondea nada: casi todo es rectangular con
      // esquinas cortadas. `full` pasa a 1px para que las barras de progreso
      // sean planas y los indicadores, pequeños cuadrados.
      borderRadius: {
        sm: "1px",
        DEFAULT: "1px",
        md: "1px",
        lg: "2px",
        xl: "2px",
        "2xl": "3px",
        "3xl": "3px",
        full: "1px",
      },
      fontFamily: {
        display: ["Cinzel", "Georgia", "Times New Roman", "serif"],
        sans: [
          "Inter",
          "Segoe UI",
          "system-ui",
          "-apple-system",
          "sans-serif",
        ],
      },
      boxShadow: {
        panel: "0 24px 48px -32px rgba(0, 0, 0, 0.95)",
        glow: "0 0 0 1px rgba(200, 155, 60, 0.45), 0 0 20px -6px rgba(200, 155, 60, 0.4)",
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
