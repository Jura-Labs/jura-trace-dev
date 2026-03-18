/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        heading: ['Georgia', 'Times New Roman', 'DejaVu Serif', 'serif'],
        body: ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', 'sans-serif'],
        mono: ['SF Mono', 'Menlo', 'Monaco', 'Courier New', 'monospace']
      },
      letterSpacing: {
        brand: '0.15em',
        nav: '0.05em',
        heading: '-0.01em',
        button: '0.02em'
      },
      colors: {
        // ===== Jura Archive Mineral Palette =====
        obsidian: {
          DEFAULT: '#1C1E26',
          light: '#2A2D37',
          dark: '#12141A'
        },
        graphite: {
          DEFAULT: '#2A2D37',
          light: '#383C48',
          dark: '#1C1E26'
        },
        quartz: {
          DEFAULT: '#E4E2DE',
          dark: '#C8C5BF'
        },
        flint: {
          DEFAULT: '#9B9890',
          light: '#B5B2AA'
        },
        lapis: {
          DEFAULT: '#3E6FA8',
          light: '#6B96CC',
          dark: '#3A6499'
        },
        malachite: {
          DEFAULT: '#5B9A6F',
          light: '#78B28A',
          dark: '#478558'
        },
        amber: {
          DEFAULT: '#D4943A',
          light: '#E0AA5C',
          dark: '#B87D2E'
        },
        cinnabar: {
          DEFAULT: '#C45B52',
          light: '#D47870',
          dark: '#A84840'
        },
        // ===== Semantic aliases =====
        surface: {
          light: '#F5F3EE',
          dark: '#1C1E26'
        },
        text: {
          light: '#1C1E26',
          dark: '#E4E2DE'
        },
        border: {
          light: '#D8D8D8',
          dark: '#383C48'
        }
      },
      typography: (theme) => ({
        DEFAULT: {
          css: {
            maxWidth: 'none',
            color: theme('colors.text.light'),
            lineHeight: '1.7',
            a: {
              color: theme('colors.lapis.DEFAULT'),
              '&:hover': {
                color: theme('colors.lapis.dark'),
              },
            },
          },
        },
        dark: {
          css: {
            color: theme('colors.text.dark'),
            a: {
              color: theme('colors.lapis.light'),
            },
          },
        },
      }),
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
